use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::{errors::DomainError, ids::StableId};

pub const DMX_SLOTS_PER_UNIVERSE: u16 = 512;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureDefinition {
    pub id: StableId,
    pub manufacturer: String,
    pub name: String,
    pub modes: Vec<FixtureMode>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureMode {
    pub id: StableId,
    pub name: String,
    pub channel_count: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureInstance {
    pub id: StableId,
    pub number: u32,
    pub name: String,
    pub definition_id: StableId,
    pub mode_id: StableId,
    pub universe: u16,
    pub start_address: u16,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Patch {
    pub fixture_definitions: Vec<FixtureDefinition>,
    pub fixtures: Vec<FixtureInstance>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PatchValidationError {
    DuplicateFixtureDefinitionId {
        definition_id: StableId,
    },
    DuplicateFixtureModeId {
        definition_id: StableId,
        mode_id: StableId,
    },
    InvalidChannelCount {
        definition_id: StableId,
        mode_id: StableId,
        channel_count: u16,
    },
    DuplicateFixtureId {
        fixture_id: StableId,
    },
    InvalidFixtureNumber {
        fixture_id: StableId,
    },
    DuplicateFixtureNumber {
        number: u32,
    },
    MissingFixtureDefinition {
        fixture_id: StableId,
        definition_id: StableId,
    },
    MissingFixtureMode {
        fixture_id: StableId,
        definition_id: StableId,
        mode_id: StableId,
    },
    InvalidUniverse {
        fixture_id: StableId,
    },
    InvalidStartAddress {
        fixture_id: StableId,
    },
    AddressOutOfBounds {
        fixture_id: StableId,
        start_address: u16,
        channel_count: u16,
    },
    AddressCollision {
        first_fixture_id: StableId,
        second_fixture_id: StableId,
        universe: u16,
        start_address: u16,
        end_address: u16,
    },
}

impl fmt::Display for PatchValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateFixtureDefinitionId { definition_id } => {
                write!(formatter, "fixture profile '{definition_id}' is duplicated")
            }
            Self::DuplicateFixtureModeId {
                definition_id,
                mode_id,
            } => write!(
                formatter,
                "mode '{mode_id}' is duplicated in fixture profile '{definition_id}'"
            ),
            Self::InvalidChannelCount {
                definition_id,
                mode_id,
                channel_count,
            } => write!(
                formatter,
                "mode '{mode_id}' in fixture profile '{definition_id}' has invalid channel count {channel_count}"
            ),
            Self::DuplicateFixtureId { fixture_id } => {
                write!(formatter, "fixture ID '{fixture_id}' is duplicated")
            }
            Self::InvalidFixtureNumber { fixture_id } => {
                write!(
                    formatter,
                    "fixture '{fixture_id}' must have a number greater than zero"
                )
            }
            Self::DuplicateFixtureNumber { number } => {
                write!(
                    formatter,
                    "fixture number {number} is assigned more than once"
                )
            }
            Self::MissingFixtureDefinition {
                fixture_id,
                definition_id,
            } => write!(
                formatter,
                "fixture '{fixture_id}' references missing profile '{definition_id}'"
            ),
            Self::MissingFixtureMode {
                fixture_id,
                definition_id,
                mode_id,
            } => write!(
                formatter,
                "fixture '{fixture_id}' references missing mode '{mode_id}' in profile '{definition_id}'"
            ),
            Self::InvalidUniverse { fixture_id } => {
                write!(
                    formatter,
                    "fixture '{fixture_id}' must use a universe greater than zero"
                )
            }
            Self::InvalidStartAddress { fixture_id } => write!(
                formatter,
                "fixture '{fixture_id}' must start at a DMX address from 1 to {DMX_SLOTS_PER_UNIVERSE}"
            ),
            Self::AddressOutOfBounds {
                fixture_id,
                start_address,
                channel_count,
            } => write!(
                formatter,
                "fixture '{fixture_id}' uses {channel_count} channels from address {start_address}, past the end of its universe"
            ),
            Self::AddressCollision {
                first_fixture_id,
                second_fixture_id,
                universe,
                start_address,
                end_address,
            } => write!(
                formatter,
                "fixtures '{first_fixture_id}' and '{second_fixture_id}' overlap in universe {universe}, addresses {start_address}-{end_address}"
            ),
        }
    }
}

#[derive(Clone, Debug)]
struct Assignment {
    fixture_id: StableId,
    universe: u16,
    start_address: u16,
    end_address: u16,
}

impl Patch {
    pub fn add_fixture(&mut self, fixture: FixtureInstance) -> Result<(), DomainError> {
        let mut fixtures = self.fixtures.clone();
        fixtures.push(fixture);
        self.replace_fixtures(fixtures)
    }

    pub fn edit_fixture(&mut self, fixture: FixtureInstance) -> Result<(), DomainError> {
        let mut fixtures = self.fixtures.clone();
        let Some(existing) = fixtures.iter_mut().find(|item| item.id == fixture.id) else {
            return Err(DomainError::FixtureNotFound {
                fixture_id: fixture.id,
            });
        };
        *existing = fixture;
        self.replace_fixtures(fixtures)
    }

    pub fn move_fixture(
        &mut self,
        fixture_id: &StableId,
        universe: u16,
        start_address: u16,
    ) -> Result<(), DomainError> {
        let mut fixtures = self.fixtures.clone();
        let Some(fixture) = fixtures.iter_mut().find(|item| &item.id == fixture_id) else {
            return Err(DomainError::FixtureNotFound {
                fixture_id: fixture_id.clone(),
            });
        };
        fixture.universe = universe;
        fixture.start_address = start_address;
        self.replace_fixtures(fixtures)
    }

    pub fn replace_fixture(
        &mut self,
        fixture_id: &StableId,
        mut replacement: FixtureInstance,
    ) -> Result<(), DomainError> {
        replacement.id = fixture_id.clone();
        self.edit_fixture(replacement)
    }

    pub fn remove_fixture(&mut self, fixture_id: &StableId) -> Result<(), DomainError> {
        let mut fixtures = self.fixtures.clone();
        let Some(index) = fixtures.iter().position(|item| &item.id == fixture_id) else {
            return Err(DomainError::FixtureNotFound {
                fixture_id: fixture_id.clone(),
            });
        };
        fixtures.remove(index);
        self.replace_fixtures(fixtures)
    }

    fn replace_fixtures(&mut self, fixtures: Vec<FixtureInstance>) -> Result<(), DomainError> {
        let mut candidate = self.clone();
        candidate.fixtures = fixtures;
        candidate.validate().map_err(DomainError::InvalidPatch)?;
        self.fixtures = candidate.fixtures;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), Vec<PatchValidationError>> {
        let mut errors = Vec::new();
        let mut definitions_by_id = HashMap::new();

        for definition in &self.fixture_definitions {
            if definitions_by_id
                .insert(&definition.id, definition)
                .is_some()
            {
                errors.push(PatchValidationError::DuplicateFixtureDefinitionId {
                    definition_id: definition.id.clone(),
                });
            }

            let mut mode_ids = HashMap::new();
            for mode in &definition.modes {
                if mode_ids.insert(&mode.id, ()).is_some() {
                    errors.push(PatchValidationError::DuplicateFixtureModeId {
                        definition_id: definition.id.clone(),
                        mode_id: mode.id.clone(),
                    });
                }

                if !(1..=DMX_SLOTS_PER_UNIVERSE).contains(&mode.channel_count) {
                    errors.push(PatchValidationError::InvalidChannelCount {
                        definition_id: definition.id.clone(),
                        mode_id: mode.id.clone(),
                        channel_count: mode.channel_count,
                    });
                }
            }
        }

        let mut fixture_ids = HashMap::new();
        let mut fixture_numbers = HashMap::new();
        let mut assignments = Vec::new();

        for fixture in &self.fixtures {
            if fixture_ids.insert(&fixture.id, ()).is_some() {
                errors.push(PatchValidationError::DuplicateFixtureId {
                    fixture_id: fixture.id.clone(),
                });
            }

            if fixture.number == 0 {
                errors.push(PatchValidationError::InvalidFixtureNumber {
                    fixture_id: fixture.id.clone(),
                });
            } else if fixture_numbers
                .insert(fixture.number, fixture.id.as_str())
                .is_some()
            {
                errors.push(PatchValidationError::DuplicateFixtureNumber {
                    number: fixture.number,
                });
            }

            let Some(definition) = definitions_by_id.get(&fixture.definition_id) else {
                errors.push(PatchValidationError::MissingFixtureDefinition {
                    fixture_id: fixture.id.clone(),
                    definition_id: fixture.definition_id.clone(),
                });
                continue;
            };

            let Some(mode) = definition
                .modes
                .iter()
                .find(|mode| mode.id == fixture.mode_id)
            else {
                errors.push(PatchValidationError::MissingFixtureMode {
                    fixture_id: fixture.id.clone(),
                    definition_id: fixture.definition_id.clone(),
                    mode_id: fixture.mode_id.clone(),
                });
                continue;
            };

            let universe_valid = fixture.universe != 0;
            if !universe_valid {
                errors.push(PatchValidationError::InvalidUniverse {
                    fixture_id: fixture.id.clone(),
                });
            }

            let address_valid = (1..=DMX_SLOTS_PER_UNIVERSE).contains(&fixture.start_address);
            if !address_valid {
                errors.push(PatchValidationError::InvalidStartAddress {
                    fixture_id: fixture.id.clone(),
                });
            }

            if !universe_valid || !address_valid || mode.channel_count == 0 {
                continue;
            }

            let end_address = u32::from(fixture.start_address) + u32::from(mode.channel_count) - 1;
            if end_address > u32::from(DMX_SLOTS_PER_UNIVERSE) {
                errors.push(PatchValidationError::AddressOutOfBounds {
                    fixture_id: fixture.id.clone(),
                    start_address: fixture.start_address,
                    channel_count: mode.channel_count,
                });
                continue;
            }

            assignments.push(Assignment {
                fixture_id: fixture.id.clone(),
                universe: fixture.universe,
                start_address: fixture.start_address,
                end_address: end_address as u16,
            });
        }

        assignments.sort_by(|left, right| {
            (
                left.universe,
                left.start_address,
                left.end_address,
                &left.fixture_id,
            )
                .cmp(&(
                    right.universe,
                    right.start_address,
                    right.end_address,
                    &right.fixture_id,
                ))
        });

        let mut active_assignments: Vec<&Assignment> = Vec::new();
        for assignment in &assignments {
            active_assignments.retain(|active| {
                active.universe == assignment.universe
                    && active.end_address >= assignment.start_address
            });

            for active in &active_assignments {
                errors.push(PatchValidationError::AddressCollision {
                    first_fixture_id: active.fixture_id.clone(),
                    second_fixture_id: assignment.fixture_id.clone(),
                    universe: assignment.universe,
                    start_address: assignment.start_address.max(active.start_address),
                    end_address: assignment.end_address.min(active.end_address),
                });
            }

            active_assignments.push(assignment);
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

    fn fixture_definition(channel_count: u16) -> FixtureDefinition {
        FixtureDefinition {
            id: id("profile"),
            manufacturer: "Iris".to_owned(),
            name: "Test fixture".to_owned(),
            modes: vec![FixtureMode {
                id: id("standard"),
                name: "Standard".to_owned(),
                channel_count,
            }],
        }
    }

    fn fixture(
        fixture_id: &str,
        number: u32,
        universe: u16,
        start_address: u16,
    ) -> FixtureInstance {
        FixtureInstance {
            id: id(fixture_id),
            number,
            name: format!("Fixture {number}"),
            definition_id: id("profile"),
            mode_id: id("standard"),
            universe,
            start_address,
        }
    }

    fn patch(channel_count: u16, fixtures: Vec<FixtureInstance>) -> Patch {
        Patch {
            fixture_definitions: vec![fixture_definition(channel_count)],
            fixtures,
        }
    }

    #[test]
    fn accepts_fixture_ending_at_last_dmx_slot() {
        assert!(
            patch(1, vec![fixture("fixture-1", 1, 1, 512)])
                .validate()
                .is_ok()
        );
        assert!(
            patch(512, vec![fixture("fixture-1", 1, 1, 1)])
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn rejects_fixture_footprint_past_last_dmx_slot() {
        let errors = patch(2, vec![fixture("fixture-1", 1, 1, 512)])
            .validate()
            .unwrap_err();

        assert!(errors.contains(&PatchValidationError::AddressOutOfBounds {
            fixture_id: id("fixture-1"),
            start_address: 512,
            channel_count: 2,
        }));
    }

    #[test]
    fn detects_overlaps_only_within_the_same_universe() {
        let result = patch(
            4,
            vec![
                fixture("fixture-1", 1, 1, 1),
                fixture("fixture-2", 2, 1, 4),
                fixture("fixture-3", 3, 2, 1),
            ],
        )
        .validate();

        assert_eq!(
            result,
            Err(vec![PatchValidationError::AddressCollision {
                first_fixture_id: id("fixture-1"),
                second_fixture_id: id("fixture-2"),
                universe: 1,
                start_address: 4,
                end_address: 4,
            }])
        );
    }

    #[test]
    fn reports_missing_definition_and_mode() {
        let mut show_patch = patch(4, vec![fixture("fixture-1", 1, 1, 1)]);
        show_patch.fixtures[0].definition_id = id("missing");

        let errors = show_patch.validate().unwrap_err();
        assert!(
            errors.contains(&PatchValidationError::MissingFixtureDefinition {
                fixture_id: id("fixture-1"),
                definition_id: id("missing"),
            })
        );

        show_patch.fixtures[0].definition_id = id("profile");
        show_patch.fixtures[0].mode_id = id("missing");
        let errors = show_patch.validate().unwrap_err();
        assert!(errors.contains(&PatchValidationError::MissingFixtureMode {
            fixture_id: id("fixture-1"),
            definition_id: id("profile"),
            mode_id: id("missing"),
        }));
    }

    #[test]
    fn rejects_invalid_numbers_universes_and_zero_channel_modes() {
        let mut show_patch = patch(0, vec![fixture("fixture-1", 0, 0, 0)]);
        let errors = show_patch.validate().unwrap_err();

        assert!(errors.contains(&PatchValidationError::InvalidChannelCount {
            definition_id: id("profile"),
            mode_id: id("standard"),
            channel_count: 0,
        }));
        assert!(
            errors.contains(&PatchValidationError::InvalidFixtureNumber {
                fixture_id: id("fixture-1"),
            })
        );
        assert!(errors.contains(&PatchValidationError::InvalidUniverse {
            fixture_id: id("fixture-1"),
        }));
        assert!(errors.contains(&PatchValidationError::InvalidStartAddress {
            fixture_id: id("fixture-1"),
        }));

        show_patch.fixtures[0].number = 1;
        assert!(show_patch.validate().is_err());
    }

    #[test]
    fn adds_and_edits_fixtures_without_changing_other_fixtures() {
        let first = fixture("fixture-1", 1, 1, 1);
        let second = fixture("fixture-2", 2, 1, 10);
        let mut show_patch = patch(4, vec![first, second.clone()]);

        let mut edited = second;
        edited.name = "Renamed".to_owned();
        edited.start_address = 20;
        show_patch.edit_fixture(edited.clone()).unwrap();

        assert_eq!(show_patch.fixtures[0], fixture("fixture-1", 1, 1, 1));
        assert_eq!(show_patch.fixtures[1], edited);

        show_patch
            .add_fixture(fixture("fixture-3", 3, 1, 30))
            .unwrap();
        assert_eq!(show_patch.fixtures.len(), 3);
    }

    #[test]
    fn rejects_invalid_fixture_changes_without_mutating_the_patch() {
        let original = patch(
            4,
            vec![
                fixture("fixture-1", 1, 1, 1),
                fixture("fixture-2", 2, 1, 10),
            ],
        );
        let mut show_patch = original.clone();

        assert!(matches!(
            show_patch.add_fixture(fixture("fixture-3", 2, 1, 20)),
            Err(DomainError::InvalidPatch(_))
        ));
        assert_eq!(show_patch, original);

        assert!(matches!(
            show_patch.move_fixture(&id("fixture-2"), 1, 4),
            Err(DomainError::InvalidPatch(_))
        ));
        assert_eq!(show_patch, original);
    }

    #[test]
    fn moves_replaces_and_removes_fixtures_by_stable_id() {
        let mut show_patch = patch(4, vec![fixture("fixture-1", 1, 1, 1)]);

        show_patch.move_fixture(&id("fixture-1"), 2, 10).unwrap();
        assert_eq!(show_patch.fixtures[0].universe, 2);
        assert_eq!(show_patch.fixtures[0].start_address, 10);

        let mut replacement = fixture("replacement-id", 7, 3, 25);
        replacement.name = "Replacement fixture".to_owned();
        show_patch
            .replace_fixture(&id("fixture-1"), replacement)
            .unwrap();

        let replaced = &show_patch.fixtures[0];
        assert_eq!(replaced.id, id("fixture-1"));
        assert_eq!(replaced.number, 7);
        assert_eq!(replaced.universe, 3);
        assert_eq!(replaced.start_address, 25);

        show_patch.remove_fixture(&id("fixture-1")).unwrap();
        assert!(show_patch.fixtures.is_empty());
    }

    #[test]
    fn fixture_changes_report_missing_ids() {
        let mut show_patch = patch(4, vec![]);
        let error = show_patch.move_fixture(&id("missing"), 1, 1).unwrap_err();

        assert_eq!(
            error,
            DomainError::FixtureNotFound {
                fixture_id: id("missing"),
            }
        );
        assert_eq!(
            show_patch.remove_fixture(&id("missing")),
            Err(DomainError::FixtureNotFound {
                fixture_id: id("missing"),
            })
        );
    }
}
