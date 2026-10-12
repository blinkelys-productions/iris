use std::fmt;

use super::{
    ids::{InvalidStableId, StableId},
    patch::PatchValidationError,
    show::ShowValidationError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainError {
    InvalidId(InvalidStableId),
    InvalidShow(Vec<ShowValidationError>),
    InvalidPatch(Vec<PatchValidationError>),
    FixtureNotFound { fixture_id: StableId },
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(error) => write!(formatter, "Invalid identifier: {error}"),
            Self::InvalidShow(errors) => write_validation_errors(formatter, "Show", errors),
            Self::InvalidPatch(errors) => write_validation_errors(formatter, "Patch", errors),
            Self::FixtureNotFound { fixture_id } => {
                write!(formatter, "Fixture '{fixture_id}' was not found")
            }
        }
    }
}

fn write_validation_errors(
    formatter: &mut fmt::Formatter<'_>,
    subject: &str,
    errors: &[impl fmt::Display],
) -> fmt::Result {
    write!(formatter, "{subject} validation failed")?;
    for (index, error) in errors.iter().enumerate() {
        write!(formatter, "{}{error}", if index == 0 { ": " } else { "; " })?;
    }
    Ok(())
}

impl std::error::Error for DomainError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_fixture_error_identifies_the_fixture() {
        let error = DomainError::FixtureNotFound {
            fixture_id: StableId::new("fixture-7").unwrap(),
        };

        assert_eq!(error.to_string(), "Fixture 'fixture-7' was not found");
    }

    #[test]
    fn invalid_patch_error_explains_the_conflict() {
        let error = DomainError::InvalidPatch(vec![PatchValidationError::AddressCollision {
            first_fixture_id: StableId::new("fixture-1").unwrap(),
            second_fixture_id: StableId::new("fixture-2").unwrap(),
            universe: 1,
            start_address: 10,
            end_address: 12,
        }]);

        assert_eq!(
            error.to_string(),
            "Patch validation failed: fixtures 'fixture-1' and 'fixture-2' overlap in universe 1, addresses 10-12"
        );
    }
}
