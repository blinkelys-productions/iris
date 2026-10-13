//! Validation and metadata extraction for uploaded GDTF archives.

use std::io::{Cursor, Read};

use zip::ZipArchive;

pub const MAX_ARCHIVE_BYTES: usize = 32 * 1024 * 1024;
const MAX_ENTRIES: usize = 2_000;
const MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DESCRIPTION_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct GdtfMeta {
    pub fixture_type_id: String,
    pub manufacturer: String,
    pub name: String,
    pub long_name: String,
    pub revision: String,
    pub modes: Vec<String>,
}

pub fn inspect(bytes: &[u8]) -> Result<GdtfMeta, String> {
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(format!("file exceeds the {} MiB limit", MAX_ARCHIVE_BYTES / 1024 / 1024));
    }
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("not a valid GDTF (zip) file: {e}"))?;
    if archive.len() > MAX_ENTRIES {
        return Err("archive contains too many entries".to_owned());
    }
    let mut total = 0u64;
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(|e| e.to_string())?;
        if file.size() > MAX_ENTRY_BYTES {
            return Err(format!("entry '{}' is too large", file.name()));
        }
        total = total.saturating_add(file.size());
        if total > MAX_TOTAL_BYTES {
            return Err("archive expands to too much data".to_owned());
        }
    }

    let mut xml = String::new();
    archive
        .by_name("description.xml")
        .map_err(|_| "archive has no description.xml".to_owned())?
        .take(MAX_DESCRIPTION_BYTES)
        .read_to_string(&mut xml)
        .map_err(|e| format!("description.xml is unreadable: {e}"))?;

    let document = roxmltree::Document::parse_with_options(
        &xml,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            ..Default::default()
        },
    )
    .map_err(|e| format!("description.xml is not valid XML: {e}"))?;

    let mut types = document
        .descendants()
        .filter(|node| node.has_tag_name("FixtureType"));
    let fixture_type = types
        .next()
        .ok_or_else(|| "GDTF defines no FixtureType".to_owned())?;
    if types.next().is_some() {
        return Err("GDTF files with several fixture types are not supported".to_owned());
    }

    let attr = |node: roxmltree::Node, name: &str| node.attribute(name).unwrap_or("").trim().to_owned();
    let name = attr(fixture_type, "Name");
    if name.is_empty() {
        return Err("FixtureType has no Name".to_owned());
    }
    let revision = fixture_type
        .descendants()
        .filter(|node| node.has_tag_name("Revision"))
        .last()
        .map(|node| attr(node, "Text"))
        .unwrap_or_default();
    let modes = fixture_type
        .descendants()
        .filter(|node| node.has_tag_name("DMXMode"))
        .map(|node| attr(node, "Name"))
        .collect();

    Ok(GdtfMeta {
        fixture_type_id: attr(fixture_type, "FixtureTypeID"),
        manufacturer: attr(fixture_type, "Manufacturer"),
        long_name: attr(fixture_type, "LongName"),
        name,
        revision,
        modes,
    })
}

#[cfg(test)]
pub fn sample_archive(name: &str) -> Vec<u8> {
    use std::io::Write;
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        zip.start_file("description.xml", zip::write::SimpleFileOptions::default())
            .unwrap();
        write!(
            zip,
            r#"<GDTF DataVersion="1.2"><FixtureType Name="{name}" Manufacturer="Acme" LongName="{name} Long" FixtureTypeID="11111111-2222-3333-4444-555555555555"><DMXModes><DMXMode Name="Standard"/><DMXMode Name="Extended"/></DMXModes><Revisions><Revision Text="v2"/></Revisions></FixtureType></GDTF>"#
        )
        .unwrap();
        zip.finish().unwrap();
    }
    cursor.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_metadata() {
        let meta = inspect(&sample_archive("Spot")).unwrap();
        assert_eq!(meta.name, "Spot");
        assert_eq!(meta.manufacturer, "Acme");
        assert_eq!(meta.modes, ["Standard", "Extended"]);
        assert_eq!(meta.revision, "v2");
    }

    #[test]
    fn rejects_garbage() {
        assert!(inspect(b"nope").is_err());
    }
}
