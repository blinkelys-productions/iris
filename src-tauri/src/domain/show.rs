use std::fmt;

use serde::{Deserialize, Serialize};

use super::{
    ids::StableId,
    patch::{Patch, PatchValidationError},
};

pub const CURRENT_SHOW_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Show {
    pub format_version: u32,
    pub id: StableId,
    pub name: String,
    pub patch: Patch,
    /// Opaque operator-interface layout (views and windows), stored and returned unchanged.
    #[serde(default)]
    pub workspace: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShowValidationError {
    EmptyShowName,
    UnsupportedShowFormatVersion { version: u32 },
    InvalidPatch(PatchValidationError),
}

impl fmt::Display for ShowValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyShowName => formatter.write_str("show name must not be empty"),
            Self::UnsupportedShowFormatVersion { version } => write!(
                formatter,
                "show format version {version} is unsupported; expected {CURRENT_SHOW_FORMAT_VERSION}"
            ),
            Self::InvalidPatch(error) => write!(formatter, "{error}"),
        }
    }
}

impl Show {
    pub fn new(id: StableId, name: impl Into<String>) -> Self {
        Self {
            format_version: CURRENT_SHOW_FORMAT_VERSION,
            id,
            name: name.into(),
            patch: Patch::default(),
            workspace: None,
        }
    }

    pub fn validate(&self) -> Result<(), Vec<ShowValidationError>> {
        let mut errors = Vec::new();

        if self.format_version != CURRENT_SHOW_FORMAT_VERSION {
            errors.push(ShowValidationError::UnsupportedShowFormatVersion {
                version: self.format_version,
            });
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

    fn id(value: &str) -> StableId {
        StableId::new(value).unwrap()
    }

    #[test]
    fn creates_a_valid_empty_show() {
        let show = Show::new(id("show-1"), "First show");

        assert_eq!(show.format_version, CURRENT_SHOW_FORMAT_VERSION);
        assert!(show.validate().is_ok());
    }

    #[test]
    fn rejects_blank_show_name() {
        let show = Show::new(id("show-1"), "\t");

        assert_eq!(
            show.validate(),
            Err(vec![ShowValidationError::EmptyShowName])
        );
    }

    #[test]
    fn rejects_unsupported_show_format_versions() {
        let mut show = Show::new(id("show-1"), "First show");
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
        let show = Show::new(id("show-1"), "First show");
        let json = serde_json::to_string(&show).unwrap();
        let restored: Show = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, show);
    }
}
