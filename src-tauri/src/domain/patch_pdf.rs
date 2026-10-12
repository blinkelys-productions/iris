use printpdf::{
    BuiltinFont, Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt, TextItem,
};

use super::patch::{FixtureInstance, Patch, PatchValidationError};

const PAGE_WIDTH_MM: f32 = 297.0;
const PAGE_HEIGHT_MM: f32 = 210.0;
const LEFT_MARGIN_MM: f32 = 10.0;
const TOP_MARGIN_MM: f32 = 12.0;
const ROW_HEIGHT_MM: f32 = 5.2;
const ROWS_PER_PAGE: usize = 32;

#[derive(Debug)]
pub enum PatchPdfError {
    InvalidPatch(Vec<PatchValidationError>),
    SerializationWarnings(Vec<String>),
}

impl std::fmt::Display for PatchPdfError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPatch(errors) => write!(
                formatter,
                "cannot export invalid patch: {}",
                errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            Self::SerializationWarnings(warnings) => write!(
                formatter,
                "cannot export patch PDF: {}",
                warnings.join("; ")
            ),
        }
    }
}

impl std::error::Error for PatchPdfError {}

pub fn export_patch_list_pdf(title: &str, patch: &Patch) -> Result<Vec<u8>, PatchPdfError> {
    patch.validate().map_err(PatchPdfError::InvalidPatch)?;

    let mut fixtures = patch.fixtures.iter().collect::<Vec<_>>();
    fixtures.sort_by(|left, right| {
        (left.universe, left.start_address, left.number, &left.id).cmp(&(
            right.universe,
            right.start_address,
            right.number,
            &right.id,
        ))
    });

    let pdf_pages = build_pages(title, patch, &fixtures);
    let mut document = PdfDocument::new(title);
    document.with_pages(pdf_pages);
    let mut warnings = Vec::new();
    let bytes = document.save(&PdfSaveOptions::default(), &mut warnings);
    if !warnings.is_empty() {
        return Err(PatchPdfError::SerializationWarnings(
            warnings
                .into_iter()
                .map(|warning| {
                    format!(
                        "page {}, operation {}: {}",
                        warning.page, warning.op_id, warning.msg
                    )
                })
                .collect(),
        ));
    }
    Ok(bytes)
}

fn build_pages(title: &str, patch: &Patch, fixtures: &[&FixtureInstance]) -> Vec<PdfPage> {
    let pages = if fixtures.is_empty() {
        1
    } else {
        fixtures.len().div_ceil(ROWS_PER_PAGE)
    };
    let mut pdf_pages = Vec::with_capacity(pages);

    for page_index in 0..pages {
        let start = page_index * ROWS_PER_PAGE;
        let end = (start + ROWS_PER_PAGE).min(fixtures.len());
        let mut operations = Vec::with_capacity(12 + (end - start) * 2);
        push_text(
            &mut operations,
            title,
            12.0,
            LEFT_MARGIN_MM,
            PAGE_HEIGHT_MM - TOP_MARGIN_MM,
            BuiltinFont::HelveticaBold,
        );
        push_text(
            &mut operations,
            &format!(
                "{} fixture(s) | Page {} of {}",
                fixtures.len(),
                page_index + 1,
                pages
            ),
            8.0,
            LEFT_MARGIN_MM,
            PAGE_HEIGHT_MM - TOP_MARGIN_MM - 7.0,
            BuiltinFont::Helvetica,
        );

        let header_y = PAGE_HEIGHT_MM - TOP_MARGIN_MM - 16.0;
        push_text(
            &mut operations,
            &format_row(
                "No.",
                "Name",
                "Manufacturer",
                "Profile",
                "Mode",
                "Universe",
                "Address",
                "Channels",
            ),
            7.0,
            LEFT_MARGIN_MM,
            header_y,
            BuiltinFont::CourierBold,
        );
        push_text(
            &mut operations,
            &"-".repeat(120),
            7.0,
            LEFT_MARGIN_MM,
            header_y - 4.5,
            BuiltinFont::Courier,
        );

        let mut y = header_y - 10.0;
        for fixture in &fixtures[start..end] {
            let definition = patch
                .fixture_definitions
                .iter()
                .find(|definition| definition.id == fixture.definition_id)
                .expect("validated patch fixture must reference a definition");
            let mode = definition
                .modes
                .iter()
                .find(|mode| mode.id == fixture.mode_id)
                .expect("validated patch fixture must reference a mode");
            push_text(
                &mut operations,
                &format_row(
                    &fixture.number.to_string(),
                    &fixture.name,
                    &definition.manufacturer,
                    &definition.name,
                    &mode.name,
                    &fixture.universe.to_string(),
                    &fixture.start_address.to_string(),
                    &mode.channel_count.to_string(),
                ),
                7.0,
                LEFT_MARGIN_MM,
                y,
                BuiltinFont::Courier,
            );
            y -= ROW_HEIGHT_MM;
        }

        pdf_pages.push(PdfPage::new(
            Mm(PAGE_WIDTH_MM),
            Mm(PAGE_HEIGHT_MM),
            operations,
        ));
    }

    pdf_pages
}

fn push_text(
    operations: &mut Vec<Op>,
    text: &str,
    font_size: f32,
    x_mm: f32,
    y_mm: f32,
    font: BuiltinFont,
) {
    operations.extend([
        Op::StartTextSection,
        Op::SetTextCursor {
            pos: Point::new(Mm(x_mm), Mm(y_mm)),
        },
        Op::SetFont {
            font: PdfFontHandle::Builtin(font),
            size: Pt(font_size),
        },
        Op::ShowText {
            items: vec![TextItem::Text(text.to_owned())],
        },
        Op::EndTextSection,
    ]);
}

fn format_row(
    number: &str,
    name: &str,
    manufacturer: &str,
    fixture_type: &str,
    mode: &str,
    universe: &str,
    address: &str,
    channels: &str,
) -> String {
    format!(
        "{:>5} {:<24} {:<18} {:<24} {:<18} {:>8} {:>8} {:>8}",
        truncate(number, 5),
        truncate(name, 24),
        truncate(manufacturer, 18),
        truncate(fixture_type, 24),
        truncate(mode, 18),
        truncate(universe, 8),
        truncate(address, 8),
        truncate(channels, 8),
    )
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ids::StableId,
        patch::{FixtureDefinition, FixtureMode},
    };

    fn id(value: &str) -> StableId {
        StableId::new(value).unwrap()
    }

    fn patch_with_fixtures(count: usize) -> Patch {
        Patch {
            fixture_definitions: vec![FixtureDefinition {
                id: id("profile"),
                manufacturer: "Iris".to_owned(),
                name: "LED Wash".to_owned(),
                modes: vec![FixtureMode {
                    id: id("basic"),
                    name: "Basic".to_owned(),
                    channel_count: 4,
                }],
            }],
            fixtures: (0..count)
                .map(|index| FixtureInstance {
                    id: id(&format!("fixture-{index}")),
                    number: index as u32 + 1,
                    name: format!("Fixture {index}"),
                    definition_id: id("profile"),
                    mode_id: id("basic"),
                    universe: 1 + (index / 128) as u16,
                    start_address: 1 + ((index % 128) * 4) as u16,
                })
                .collect(),
        }
    }

    #[test]
    fn exports_a_valid_pdf_for_empty_patch() {
        let bytes = export_patch_list_pdf("Empty show", &Patch::default()).unwrap();

        assert!(bytes.starts_with(b"%PDF-"));
        assert!(bytes.ends_with(b"%%EOF") || bytes.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn paginates_fixture_rows() {
        let patch = patch_with_fixtures(33);
        let fixtures = patch.fixtures.iter().collect::<Vec<_>>();
        let pages = build_pages("Large show", &patch, &fixtures);

        assert_eq!(pages.len(), 2);
        assert!(pages[0].ops.iter().any(|operation| matches!(
            operation,
            Op::ShowText { items }
                if items.iter().any(|item| matches!(item, TextItem::Text(text) if text.contains("Page 1 of 2")))
        )));
        assert!(pages[1].ops.iter().any(|operation| matches!(
            operation,
            Op::ShowText { items }
                if items.iter().any(|item| matches!(item, TextItem::Text(text) if text.contains("Page 2 of 2")))
        )));
        assert!(pages[0].ops.iter().any(|operation| matches!(
            operation,
            Op::ShowText { items }
                if items.iter().any(|item| matches!(item, TextItem::Text(text) if text.contains("Fixture 0") && text.contains("Iris") && text.contains("LED Wash") && text.contains("Basic")))
        )));
        assert!(pages[1].ops.iter().any(|operation| matches!(
            operation,
            Op::ShowText { items }
                if items.iter().any(|item| matches!(item, TextItem::Text(text) if text.contains("Fixture 32")))
        )));
    }

    #[test]
    fn refuses_to_export_invalid_patches() {
        let mut patch = patch_with_fixtures(2);
        patch.fixtures[1].number = patch.fixtures[0].number;

        assert!(matches!(
            export_patch_list_pdf("Invalid", &patch),
            Err(PatchPdfError::InvalidPatch(_))
        ));
    }

    #[test]
    fn truncates_long_values_without_breaking_row_width() {
        let row = format_row(
            "1",
            &"n".repeat(40),
            "Maker",
            "Fixture",
            "Mode",
            "1",
            "1",
            "10",
        );

        assert_eq!(
            row.len(),
            5 + 1 + 24 + 1 + 18 + 1 + 24 + 1 + 18 + 1 + 8 + 1 + 8 + 1 + 8
        );
        assert!(row.contains(&"n".repeat(24)));
    }
}
