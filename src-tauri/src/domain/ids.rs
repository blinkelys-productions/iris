use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// An opaque identifier that callers persist unchanged for the entity's lifetime.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String")]
pub struct StableId(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidStableId;

impl StableId {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidStableId> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(InvalidStableId);
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for StableId {
    type Error = InvalidStableId;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for StableId {
    type Error = InvalidStableId;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl FromStr for StableId {
    type Err = InvalidStableId;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl From<StableId> for String {
    fn from(id: StableId) -> Self {
        id.0
    }
}

impl fmt::Display for StableId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl fmt::Display for InvalidStableId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("stable IDs must not be empty or whitespace")
    }
}

impl std::error::Error for InvalidStableId {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_and_preserves_non_empty_ids() {
        let id = StableId::new("fixture-01").unwrap();
        assert_eq!(id.as_str(), "fixture-01");
    }

    #[test]
    fn rejects_empty_ids_on_creation_and_deserialization() {
        assert!(StableId::new(" \t").is_err());
        assert!(serde_json::from_str::<StableId>(r#""""#).is_err());
    }

    #[test]
    fn serializes_as_a_string() {
        let id = StableId::new("fixture-01").unwrap();
        assert_eq!(serde_json::to_string(&id).unwrap(), r#""fixture-01""#);
    }
}
