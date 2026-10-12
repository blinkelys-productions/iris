use serde::{Deserialize, Serialize};

use super::patch::{Patch, PatchValidationError};

pub const CURRENT_SHOW_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Show {
    pub format_version: u32,
    pub id: String,
    pub name: String,
    pub patch: Patch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShowValidationError {
    EmptyShowId,
    EmptyShowName,
    UnsupportedShowFormatVersion { version: u32 },
    InvalidPatch(PatchValidationError),
}

impl Show {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            format_version: CURRENT_SHOW_FORMAT_VERSION,
            id: id.into(),
            name: name.into(),
            patch: Patch::default(),
        }
    }

    pub fn validate(&self) -> Result<(), Vec<ShowValidationError>> {
        let mut errors = Vec::new();

        if self.format_version != CURRENT_SHOW_FORMAT_VERSION {
            errors.push(ShowValidationError::UnsupportedShowFormatVersion {
                version: self.format_version,
            });
        }

        if self.id.trim().is_empty() {
            errors.push(ShowValidationError::EmptyShowId);
        }

        if self.name.trim().is_empty() {
            errors.push(ShowValidationError::EmptyShowName);
        }

        if let Err(patch_errors) = self.patch.validate() {
            errors.extend(
                patch_errors
                    .into_iter()
                    .map(ShowValidationError::InvalidPatch),
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_valid_empty_show() {
        let show = Show::new("show-1", "First show");

        assert_eq!(show.format_version, CURRENT_SHOW_FORMAT_VERSION);
        assert!(show.validate().is_ok());
    }

    #[test]
    fn rejects_blank_show_identity_and_name() {
        let show = Show::new("  ", "\t");

        assert_eq!(
            show.validate(),
            Err(vec![
                ShowValidationError::EmptyShowId,
                ShowValidationError::EmptyShowName,
            ])
        );
    }

    #[test]
    fn rejects_unsupported_show_format_versions() {
        let mut show = Show::new("show-1", "First show");
        show.format_version += 1;

        assert_eq!(
            show.validate(),
            Err(vec![ShowValidationError::UnsupportedShowFormatVersion {
                version: show.format_version,
            }])
        );
    }

    #[test]
    fn serializes_and_restores_persistent_show_data() {
        let show = Show::new("show-1", "First show");
        let json = serde_json::to_string(&show).unwrap();
        let restored: Show = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, show);
    }
}
