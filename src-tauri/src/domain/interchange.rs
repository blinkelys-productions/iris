//! GDTF profile and MVR patch interchange.
//!
//! The current show model represents patch addresses but not MVR scene geometry or
//! multi-break fixtures. MVR import therefore returns an explicit scene-data warning.

use std::{
    collections::BTreeMap,
    fmt,
    io::{Cursor, Read, Write},
};

use gdtf::{GdtfFile, dmx_mode::DmxBreak};
use roxmltree::{Document, Node};
use uuid::Uuid;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use super::{
    ids::{InvalidStableId, StableId},
    patch::{FixtureDefinition, FixtureInstance, FixtureMode, Patch, PatchValidationError},
};

const MAX_ARCHIVE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 10_000;
const MAX_DESCRIPTION_BYTES: u64 = 16 * 1024 * 1024;
const MVR_XML_NAME: &str = "GeneralSceneDescription.xml";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedGdtf {
    pub definition: FixtureDefinition,
    pub archive_file_name: String,
    pub archive: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InterchangeWarning {
    MvrSceneDataNotRepresented,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedMvr {
    pub patch: Patch,
    pub gdtf_archives: BTreeMap<StableId, ImportedGdtf>,
    pub warnings: Vec<InterchangeWarning>,
}

#[derive(Debug)]
pub enum InterchangeError {
    Io(std::io::Error),
    Zip(zip::result::ZipError),
    Gdtf(gdtf::GdtfError),
    Xml(roxmltree::Error),
    InvalidArchive(String),
    InvalidFormat(String),
    UnsupportedFeature(String),
    InvalidId(InvalidStableId),
    InvalidPatch(Vec<PatchValidationError>),
}

impl fmt::Display for InterchangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "interchange I/O failed: {error}"),
            Self::Zip(error) => write!(formatter, "invalid interchange archive: {error}"),
            Self::Gdtf(error) => write!(formatter, "invalid GDTF profile: {error}"),
            Self::Xml(error) => write!(formatter, "invalid MVR XML: {error}"),
            Self::InvalidArchive(message) => write!(formatter, "invalid archive: {message}"),
            Self::InvalidFormat(message) => write!(formatter, "invalid format: {message}"),
            Self::UnsupportedFeature(message) => {
                write!(formatter, "unsupported interchange feature: {message}")
            }
            Self::InvalidId(error) => write!(formatter, "invalid stable ID: {error}"),
            Self::InvalidPatch(errors) => write!(
                formatter,
                "patch validation failed: {}",
                errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        }
    }
}

impl std::error::Error for InterchangeError {}

impl From<std::io::Error> for InterchangeError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<zip::result::ZipError> for InterchangeError {
    fn from(error: zip::result::ZipError) -> Self {
        Self::Zip(error)
    }
}

impl From<gdtf::GdtfError> for InterchangeError {
    fn from(error: gdtf::GdtfError) -> Self {
        Self::Gdtf(error)
    }
}

impl From<roxmltree::Error> for InterchangeError {
    fn from(error: roxmltree::Error) -> Self {
        Self::Xml(error)
    }
}

impl From<InvalidStableId> for InterchangeError {
    fn from(error: InvalidStableId) -> Self {
        Self::InvalidId(error)
    }
}

pub fn import_gdtf(
    archive_file_name: impl Into<String>,
    bytes: Vec<u8>,
) -> Result<ImportedGdtf, InterchangeError> {
    validate_zip_limits(&bytes, Some("description.xml"))?;

    let parsed = GdtfFile::new(Cursor::new(bytes.clone()))?;
    let mut fixture_types = parsed.description.fixture_types.iter();
    let Some(fixture_type) = fixture_types.next() else {
        return Err(InterchangeError::InvalidFormat(
            "GDTF archive defines no fixture types".to_owned(),
        ));
    };
    if fixture_types.next().is_some() {
        return Err(InterchangeError::UnsupportedFeature(
            "a GDTF archive containing multiple fixture types".to_owned(),
        ));
    }

    let type_name = fixture_type
        .name
        .as_ref()
        .map(|name| name.as_ref())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(fixture_type.long_name.as_str());
    let definition_id = StableId::new(format!("gdtf:{}", fixture_type.fixture_type_id))?;
    let mut modes = Vec::with_capacity(fixture_type.dmx_modes.len());

    for mode in &fixture_type.dmx_modes {
        let Some(mode_name) = mode.name.as_ref().map(|name| name.as_ref()) else {
            return Err(InterchangeError::InvalidFormat(
                "GDTF contains a DMX mode without a name".to_owned(),
            ));
        };

        let mut channel_count = 0u16;
        for channel in &mode.dmx_channels {
            match channel.dmx_break {
                DmxBreak::Value(1) => {}
                DmxBreak::Value(value) => {
                    return Err(InterchangeError::UnsupportedFeature(format!(
                        "GDTF mode '{mode_name}' uses DMX break {value}; the patch model supports one break"
                    )));
                }
                DmxBreak::Overwrite => {
                    return Err(InterchangeError::UnsupportedFeature(format!(
                        "GDTF mode '{mode_name}' uses an inherited DMX break"
                    )));
                }
            }
            for offset in channel.offset.iter().flatten() {
                if *offset < 1 || *offset > i32::from(super::patch::DMX_SLOTS_PER_UNIVERSE) {
                    return Err(InterchangeError::InvalidFormat(format!(
                        "GDTF mode '{mode_name}' has an invalid DMX offset {offset}"
                    )));
                }
                channel_count = channel_count.max(*offset as u16);
            }
        }
        if channel_count == 0 {
            return Err(InterchangeError::UnsupportedFeature(format!(
                "GDTF mode '{mode_name}' has no addressed DMX channels"
            )));
        }

        modes.push(FixtureMode {
            id: StableId::new(format!("{definition_id}:mode:{mode_name}"))?,
            name: mode_name.to_owned(),
            channel_count,
        });
    }

    if modes.is_empty() {
        return Err(InterchangeError::InvalidFormat(
            "GDTF fixture type defines no DMX modes".to_owned(),
        ));
    }

    Ok(ImportedGdtf {
        definition: FixtureDefinition {
            id: definition_id,
            manufacturer: fixture_type.manufacturer.clone(),
            name: type_name.to_owned(),
            modes,
        },
        archive_file_name: archive_file_name.into(),
        archive: bytes,
    })
}

pub fn export_gdtf(imported: &ImportedGdtf) -> &[u8] {
    &imported.archive
}

pub fn import_mvr(bytes: &[u8]) -> Result<ImportedMvr, InterchangeError> {
    validate_zip_limits(bytes, Some(MVR_XML_NAME))?;
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    let mut xml = String::new();
    archive
        .by_name(MVR_XML_NAME)?
        .take(MAX_DESCRIPTION_BYTES + 1)
        .read_to_string(&mut xml)?;
    if xml.len() as u64 > MAX_DESCRIPTION_BYTES {
        return Err(InterchangeError::InvalidArchive(format!(
            "{MVR_XML_NAME} exceeds the supported size limit"
        )));
    }

    let document = Document::parse(&xml)?;
    let root = document.root_element();
    if root.tag_name().name() != "GeneralSceneDescription" {
        return Err(InterchangeError::InvalidFormat(
            "MVR root element must be GeneralSceneDescription".to_owned(),
        ));
    }
    let major = root
        .attribute("verMajor")
        .unwrap_or("1")
        .parse::<u16>()
        .map_err(|_| {
            InterchangeError::InvalidFormat("MVR has an invalid major version".to_owned())
        })?;
    let minor = root
        .attribute("verMinor")
        .unwrap_or("0")
        .parse::<u16>()
        .map_err(|_| {
            InterchangeError::InvalidFormat("MVR has an invalid minor version".to_owned())
        })?;
    if major != 1 || !(4..=6).contains(&minor) {
        return Err(InterchangeError::UnsupportedFeature(format!(
            "MVR version {major}.{minor}"
        )));
    }

    let mut patch = Patch::default();
    let mut imported_profiles = BTreeMap::<StableId, ImportedGdtf>::new();
    for fixture_node in root
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "Fixture")
    {
        let fixture_uuid = required_attribute(fixture_node, "uuid")?;
        let fixture_id = Uuid::parse_str(fixture_uuid).map_err(|_| {
            InterchangeError::InvalidFormat(format!("fixture has invalid UUID '{fixture_uuid}'"))
        })?;
        let fixture_id = StableId::new(fixture_id.to_string())?;
        let fixture_name = required_attribute(fixture_node, "name")?.to_owned();
        let gdtf_name = required_child_text(fixture_node, "GDTFSpec")?.to_owned();
        let mode_name = required_child_text(fixture_node, "GDTFMode")?;
        let fixture_number = required_child_text(fixture_node, "FixtureID")?
            .parse::<u32>()
            .map_err(|_| {
                InterchangeError::InvalidFormat(format!(
                    "fixture '{fixture_name}' has an invalid FixtureID"
                ))
            })?;

        let addresses = fixture_node
            .children()
            .find(|node| node.is_element() && node.tag_name().name() == "Addresses")
            .ok_or_else(|| {
                InterchangeError::InvalidFormat(format!(
                    "fixture '{fixture_name}' has no Addresses element"
                ))
            })?;
        let mut address_nodes = addresses
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "Address");
        let address_node = address_nodes.next().ok_or_else(|| {
            InterchangeError::InvalidFormat(format!("fixture '{fixture_name}' has no DMX Address"))
        })?;
        if address_nodes.next().is_some() {
            return Err(InterchangeError::UnsupportedFeature(format!(
                "fixture '{fixture_name}' uses multiple DMX breaks"
            )));
        }
        let break_number = address_node
            .attribute("break")
            .unwrap_or("0")
            .parse::<u16>()
            .map_err(|_| {
                InterchangeError::InvalidFormat(format!(
                    "fixture '{fixture_name}' has an invalid DMX break"
                ))
            })?;
        let universe = break_number.checked_add(1).ok_or_else(|| {
            InterchangeError::InvalidFormat(format!(
                "fixture '{fixture_name}' has an out-of-range DMX break"
            ))
        })?;
        let start_address = node_text(address_node)?.parse::<u16>().map_err(|_| {
            InterchangeError::InvalidFormat(format!(
                "fixture '{fixture_name}' has an invalid DMX address"
            ))
        })?;

        let profile = if let Some(profile) = imported_profiles
            .values()
            .find(|profile| profile.archive_file_name == gdtf_name)
        {
            profile.clone()
        } else {
            let gdtf_bytes = read_archive_entry(&mut archive, &gdtf_name)?;
            let imported = import_gdtf(gdtf_name.clone(), gdtf_bytes)?;
            if let Some(existing) = imported_profiles.get(&imported.definition.id) {
                if existing.archive != imported.archive {
                    return Err(InterchangeError::InvalidFormat(format!(
                        "different GDTF archives declare the same fixture type ID '{}'",
                        imported.definition.id
                    )));
                }
            } else {
                patch.fixture_definitions.push(imported.definition.clone());
                imported_profiles.insert(imported.definition.id.clone(), imported.clone());
            }
            imported
        };

        let mode = profile
            .definition
            .modes
            .iter()
            .find(|mode| mode.name == mode_name)
            .ok_or_else(|| {
                InterchangeError::InvalidFormat(format!(
                    "fixture '{fixture_name}' references unknown mode '{mode_name}' in '{}'",
                    profile.definition.name
                ))
            })?;

        patch.fixtures.push(FixtureInstance {
            id: fixture_id,
            number: fixture_number,
            name: fixture_name,
            definition_id: profile.definition.id.clone(),
            mode_id: mode.id.clone(),
            universe,
            start_address,
        });
    }

    patch.validate().map_err(InterchangeError::InvalidPatch)?;

    let warnings = if contains_unrepresented_scene_data(root) {
        vec![InterchangeWarning::MvrSceneDataNotRepresented]
    } else {
        Vec::new()
    };
    Ok(ImportedMvr {
        patch,
        gdtf_archives: imported_profiles,
        warnings,
    })
}

pub fn export_mvr(
    patch: &Patch,
    gdtf_archives: &BTreeMap<StableId, Vec<u8>>,
) -> Result<Vec<u8>, InterchangeError> {
    patch.validate().map_err(InterchangeError::InvalidPatch)?;

    let mut profile_files = BTreeMap::new();
    for definition in &patch.fixture_definitions {
        let Some(bytes) = gdtf_archives.get(&definition.id) else {
            if patch
                .fixtures
                .iter()
                .any(|fixture| fixture.definition_id == definition.id)
            {
                return Err(InterchangeError::InvalidFormat(format!(
                    "no GDTF archive was supplied for fixture profile '{}'",
                    definition.id
                )));
            }
            continue;
        };
        let parsed = import_gdtf("profile.gdtf", bytes.clone())?;
        if parsed.definition.id != definition.id {
            return Err(InterchangeError::InvalidFormat(format!(
                "GDTF archive profile ID '{}' does not match patch profile ID '{}'",
                parsed.definition.id, definition.id
            )));
        }
        if parsed.definition != *definition {
            return Err(InterchangeError::InvalidFormat(format!(
                "GDTF archive for profile '{}' does not match the profile stored in the patch",
                definition.id
            )));
        }
        profile_files.insert(
            definition.id.clone(),
            (
                gdtf_file_name(&definition.manufacturer, &definition.name, &definition.id),
                bytes,
            ),
        );
    }

    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <GeneralSceneDescription verMajor=\"1\" verMinor=\"6\">\n\
         <Scene><Layers><Layer name=\"Iris\" uuid=\"",
    );
    xml.push_str(&mvr_uuid("layer:iris"));
    xml.push_str("\"><ChildList>");
    for fixture in &patch.fixtures {
        let definition = patch
            .fixture_definitions
            .iter()
            .find(|definition| definition.id == fixture.definition_id)
            .ok_or_else(|| {
                InterchangeError::InvalidFormat(format!(
                    "fixture '{}' has no fixture profile",
                    fixture.id
                ))
            })?;
        let mode = definition
            .modes
            .iter()
            .find(|mode| mode.id == fixture.mode_id)
            .ok_or_else(|| {
                InterchangeError::InvalidFormat(format!(
                    "fixture '{}' has no selected mode",
                    fixture.id
                ))
            })?;
        let (gdtf_name, _) = profile_files.get(&definition.id).ok_or_else(|| {
            InterchangeError::InvalidFormat(format!(
                "no GDTF archive was supplied for fixture profile '{}'",
                definition.id
            ))
        })?;

        xml.push_str("<Fixture name=\"");
        xml.push_str(&xml_escape(&fixture.name));
        xml.push_str("\" uuid=\"");
        xml.push_str(&mvr_uuid(fixture.id.as_str()));
        xml.push_str("\"><Matrix>1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1</Matrix><GDTFSpec>");
        xml.push_str(&xml_escape(gdtf_name));
        xml.push_str("</GDTFSpec><GDTFMode>");
        xml.push_str(&xml_escape(&mode.name));
        xml.push_str("</GDTFMode><Addresses><Address break=\"");
        xml.push_str(&(fixture.universe - 1).to_string());
        xml.push_str("\">");
        xml.push_str(&fixture.start_address.to_string());
        xml.push_str("</Address></Addresses><FixtureID>");
        xml.push_str(&fixture.number.to_string());
        xml.push_str("</FixtureID></Fixture>");
    }
    xml.push_str("</ChildList></Layer></Layers></Scene></GeneralSceneDescription>");

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    writer.start_file(MVR_XML_NAME, options)?;
    writer.write_all(xml.as_bytes())?;
    for (_, (name, bytes)) in profile_files {
        writer.start_file(name, options)?;
        writer.write_all(bytes)?;
    }
    let bytes = writer.finish()?.into_inner();
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(InterchangeError::InvalidArchive(format!(
            "exported MVR exceeds the {} MiB archive size limit",
            MAX_ARCHIVE_BYTES / (1024 * 1024)
        )));
    }
    Ok(bytes)
}

fn validate_zip_limits(bytes: &[u8], required_entry: Option<&str>) -> Result<(), InterchangeError> {
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(InterchangeError::InvalidArchive(format!(
            "archive exceeds the {} MiB limit",
            MAX_ARCHIVE_BYTES / (1024 * 1024)
        )));
    }

    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(InterchangeError::InvalidArchive(format!(
            "archive contains more than {MAX_ARCHIVE_ENTRIES} entries"
        )));
    }
    let mut total_uncompressed_bytes = 0u64;
    let mut required_entry_found = required_entry.is_none();
    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        if file.size() > MAX_ENTRY_BYTES {
            return Err(InterchangeError::InvalidArchive(format!(
                "entry '{}' exceeds the supported expanded size",
                file.name()
            )));
        }
        total_uncompressed_bytes = total_uncompressed_bytes
            .checked_add(file.size())
            .ok_or_else(|| {
                InterchangeError::InvalidArchive(
                    "archive expanded size overflows the supported range".to_owned(),
                )
            })?;
        if total_uncompressed_bytes > MAX_TOTAL_UNCOMPRESSED_BYTES {
            return Err(InterchangeError::InvalidArchive(
                "archive exceeds the total expanded size limit".to_owned(),
            ));
        }
        if Some(file.name()) == required_entry {
            required_entry_found = true;
            if file.size() > MAX_DESCRIPTION_BYTES {
                return Err(InterchangeError::InvalidArchive(format!(
                    "entry '{}' exceeds the supported description size",
                    file.name()
                )));
            }
        }
    }
    if !required_entry_found {
        return Err(InterchangeError::InvalidArchive(format!(
            "required archive entry '{}' is missing",
            required_entry.unwrap_or_default()
        )));
    }
    Ok(())
}

fn read_archive_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Result<Vec<u8>, InterchangeError> {
    if name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err(InterchangeError::InvalidFormat(format!(
            "MVR references an invalid GDTF archive path '{name}'"
        )));
    }
    let mut file = archive.by_name(name)?;
    if file.size() > MAX_ENTRY_BYTES {
        return Err(InterchangeError::InvalidArchive(format!(
            "embedded GDTF '{name}' exceeds the supported expanded size"
        )));
    }
    let mut bytes = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn required_attribute<'a>(node: Node<'a, 'a>, name: &str) -> Result<&'a str, InterchangeError> {
    node.attribute(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            InterchangeError::InvalidFormat(format!(
                "MVR <{}> is missing the '{name}' attribute",
                node.tag_name().name()
            ))
        })
}

fn required_child_text<'a>(
    node: Node<'a, 'a>,
    child_name: &str,
) -> Result<&'a str, InterchangeError> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == child_name)
        .map(node_text)
        .transpose()?
        .filter(|text| !text.is_empty())
        .ok_or_else(|| {
            InterchangeError::InvalidFormat(format!(
                "MVR <{}> is missing <{child_name}>",
                node.tag_name().name()
            ))
        })
}

fn node_text<'a>(node: Node<'a, 'a>) -> Result<&'a str, InterchangeError> {
    node.text()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| {
            InterchangeError::InvalidFormat(format!(
                "MVR <{}> has no text content",
                node.tag_name().name()
            ))
        })
}

fn contains_unrepresented_scene_data(root: Node<'_, '_>) -> bool {
    const REPRESENTED_ELEMENTS: &[&str] = &[
        "GeneralSceneDescription",
        "Scene",
        "Layers",
        "Layer",
        "ChildList",
        "Fixture",
        "GDTFSpec",
        "GDTFMode",
        "FixtureID",
        "Addresses",
        "Address",
    ];

    root.descendants()
        .any(|node| node.is_element() && !REPRESENTED_ELEMENTS.contains(&node.tag_name().name()))
}

fn gdtf_file_name(manufacturer: &str, name: &str, id: &StableId) -> String {
    format!(
        "{}@{}@{}.gdtf",
        safe_filename_part(manufacturer),
        safe_filename_part(name),
        safe_filename_part(id.as_str())
    )
}

fn safe_filename_part(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn mvr_uuid(stable_id: &str) -> String {
    Uuid::parse_str(stable_id)
        .unwrap_or_else(|_| Uuid::new_v5(&Uuid::NAMESPACE_URL, stable_id.as_bytes()))
        .to_string()
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_GDTF_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<GDTF DataVersion="1.2">
  <FixtureType Name="TestFixture" ShortName="Test" LongName="Test Fixture" Manufacturer="Iris" Description="" FixtureTypeID="e6e44f65-4f1d-4b62-b614-9ab1f6c0c2d1" CanHaveChildren="No">
    <AttributeDefinitions><ActivationGroups/><FeatureGroups/><Attributes/></AttributeDefinitions>
    <Wheels/>
    <PhysicalDescriptions><ColorSpace Mode="sRGB"/><Filters/><Emitters/><DMXProfiles/><CRIs/><Connectors/><Properties/></PhysicalDescriptions>
    <Models/>
    <Geometries><Geometry Name="Body" Model="Body" Position="{1,0,0,0}{0,1,0,0}{0,0,1,0}{0,0,0,1}"/></Geometries>
    <DMXModes>
      <DMXMode Name="Basic" Geometry="Body">
        <DMXChannels>
          <DMXChannel DMXBreak="1" Geometry="Body" Highlight="0/1" Offset="1">
            <LogicalChannel Attribute="Dimmer"><ChannelFunction Attribute="Dimmer" DMXFrom="0/1" Default="0/1"/></LogicalChannel>
          </DMXChannel>
          <DMXChannel DMXBreak="1" Geometry="Body" Highlight="0/1" Offset="2">
            <LogicalChannel Attribute="ColorAdd_R"><ChannelFunction Attribute="ColorAdd_R" DMXFrom="0/1" Default="0/1"/></LogicalChannel>
          </DMXChannel>
        </DMXChannels>
        <Relations/><FTMacros/>
      </DMXMode>
    </DMXModes>
    <Revisions/><FTPresets/><Protocols/>
  </FixtureType>
</GDTF>"#;

    fn gdtf_archive() -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(
                "description.xml",
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(MINIMAL_GDTF_XML.as_bytes()).unwrap();
        writer.finish().unwrap().into_inner()
    }

    fn mvr_archive(xml: &str) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(
                MVR_XML_NAME,
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(xml.as_bytes()).unwrap();
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn exports_and_imports_an_empty_mvr_patch() {
        let bytes = export_mvr(&Patch::default(), &BTreeMap::new()).unwrap();
        let imported = import_mvr(&bytes).unwrap();

        assert!(imported.patch.fixtures.is_empty());
        assert!(imported.patch.fixture_definitions.is_empty());
        assert_eq!(imported.warnings, Vec::<InterchangeWarning>::new());
    }

    #[test]
    fn rejects_unsupported_mvr_versions() {
        let bytes = mvr_archive(
            r#"<GeneralSceneDescription verMajor="2" verMinor="0"><Scene/></GeneralSceneDescription>"#,
        );

        assert!(matches!(
            import_mvr(&bytes),
            Err(InterchangeError::UnsupportedFeature(_))
        ));
    }

    #[test]
    fn rejects_mvr_fixture_resource_paths_outside_the_archive_root() {
        let bytes = mvr_archive(
            r#"<GeneralSceneDescription verMajor="1" verMinor="6"><Scene><Fixture uuid="e6e44f65-4f1d-4b62-b614-9ab1f6c0c2d1" name="Spot"><GDTFSpec>../outside.gdtf</GDTFSpec><GDTFMode>Standard</GDTFMode><FixtureID>1</FixtureID><Addresses><Address break="0">1</Address></Addresses></Fixture></Scene></GeneralSceneDescription>"#,
        );

        assert!(matches!(
            import_mvr(&bytes),
            Err(InterchangeError::InvalidFormat(message))
                if message.contains("invalid GDTF archive path")
        ));
    }

    #[test]
    fn rejects_non_zip_gdtf_input() {
        assert!(matches!(
            import_gdtf("bad.gdtf", b"not a zip".to_vec()),
            Err(InterchangeError::Zip(_))
        ));
    }

    #[test]
    fn imports_gdtf_profile_and_preserves_archive_bytes_for_export() {
        let bytes = gdtf_archive();
        let imported = import_gdtf("test.gdtf", bytes.clone()).unwrap();

        assert_eq!(imported.definition.manufacturer, "Iris");
        assert_eq!(imported.definition.name, "TestFixture");
        assert_eq!(imported.definition.modes[0].name, "Basic");
        assert_eq!(imported.definition.modes[0].channel_count, 2);
        assert_eq!(export_gdtf(&imported), bytes);
    }

    #[test]
    fn mvr_patch_round_trip_embeds_and_restores_gdtf_archive() {
        let archive = gdtf_archive();
        let imported_profile = import_gdtf("test.gdtf", archive.clone()).unwrap();
        let definition = imported_profile.definition.clone();
        let mode = definition.modes[0].clone();
        let patch = Patch {
            fixture_definitions: vec![definition.clone()],
            fixtures: vec![FixtureInstance {
                id: StableId::new("fixture-1").unwrap(),
                number: 21,
                name: "Front & center".to_owned(),
                definition_id: definition.id.clone(),
                mode_id: mode.id,
                universe: 4,
                start_address: 128,
            }],
        };
        let bytes = export_mvr(
            &patch,
            &BTreeMap::from([(definition.id.clone(), archive.clone())]),
        )
        .unwrap();
        let restored = import_mvr(&bytes).unwrap();

        assert_eq!(restored.patch.fixture_definitions, vec![definition.clone()]);
        assert_eq!(restored.patch.fixtures.len(), 1);
        let fixture = &restored.patch.fixtures[0];
        assert_eq!(fixture.number, 21);
        assert_eq!(fixture.name, "Front & center");
        assert_eq!(fixture.definition_id, definition.id);
        assert_eq!(fixture.mode_id, patch.fixtures[0].mode_id);
        assert_eq!(fixture.universe, 4);
        assert_eq!(fixture.start_address, 128);
        assert_eq!(fixture.id.as_str(), mvr_uuid("fixture-1"));
        assert_eq!(
            restored.gdtf_archives[&fixture.definition_id].archive,
            archive
        );
    }

    #[test]
    fn refuses_to_export_a_fixture_without_its_gdtf_archive() {
        let imported_profile = import_gdtf("test.gdtf", gdtf_archive()).unwrap();
        let definition = imported_profile.definition;
        let patch = Patch {
            fixture_definitions: vec![definition.clone()],
            fixtures: vec![FixtureInstance {
                id: StableId::new("fixture-1").unwrap(),
                number: 1,
                name: "Test".to_owned(),
                definition_id: definition.id.clone(),
                mode_id: definition.modes[0].id.clone(),
                universe: 1,
                start_address: 1,
            }],
        };

        assert!(matches!(
            export_mvr(&patch, &BTreeMap::new()),
            Err(InterchangeError::InvalidFormat(message))
                if message.contains("no GDTF archive")
        ));
    }

    #[test]
    fn refuses_to_export_when_the_gdtf_archive_does_not_match_the_patch_profile() {
        let archive = gdtf_archive();
        let mut imported_profile = import_gdtf("test.gdtf", archive.clone()).unwrap();
        imported_profile.definition.name = "Renamed".to_owned();
        let patch = Patch {
            fixture_definitions: vec![imported_profile.definition.clone()],
            fixtures: vec![FixtureInstance {
                id: StableId::new("fixture-1").unwrap(),
                number: 1,
                name: "Test".to_owned(),
                definition_id: imported_profile.definition.id.clone(),
                mode_id: imported_profile.definition.modes[0].id.clone(),
                universe: 1,
                start_address: 1,
            }],
        };

        assert!(matches!(
            export_mvr(
                &patch,
                &BTreeMap::from([(imported_profile.definition.id, archive)])
            ),
            Err(InterchangeError::InvalidFormat(message))
                if message.contains("does not match the profile stored")
        ));
    }

    #[test]
    fn reports_scene_objects_that_are_not_represented_by_the_patch_model() {
        let bytes = mvr_archive(
            r#"<GeneralSceneDescription verMajor="1" verMinor="6"><Scene><Layers><Layer name="Main" uuid="e6e44f65-4f1d-4b62-b614-9ab1f6c0c2d1"><ChildList><Truss name="Truss" uuid="e6e44f65-4f1d-4b62-b614-9ab1f6c0c2d2"/></ChildList></Layer></Layers></Scene></GeneralSceneDescription>"#,
        );

        let imported = import_mvr(&bytes).unwrap();
        assert_eq!(
            imported.warnings,
            vec![InterchangeWarning::MvrSceneDataNotRepresented]
        );
    }
}
