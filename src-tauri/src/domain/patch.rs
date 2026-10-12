use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const DMX_SLOTS_PER_UNIVERSE: u16 = 512;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureDefinition {
    pub id: String,
    pub manufacturer: String,
    pub name: String,
    pub modes: Vec<FixtureMode>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureMode {
    pub id: String,
    pub name: String,
    pub channel_count: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureInstance {
    pub id: String,
    pub number: u32,
    pub name: String,
    pub definition_id: String,
    pub mode_id: String,
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
    EmptyFixtureDefinitionId,
    EmptyFixtureModeId {
        definition_id: String,
    },
    EmptyFixtureId,
    DuplicateFixtureDefinitionId {
        definition_id: String,
    },
    DuplicateFixtureModeId {
        definition_id: String,
        mode_id: String,
    },
    InvalidChannelCount {
        definition_id: String,
        mode_id: String,
        channel_count: u16,
    },
    DuplicateFixtureId {
        fixture_id: String,
    },
    InvalidFixtureNumber {
        fixture_id: String,
    },
    DuplicateFixtureNumber {
        number: u32,
    },
    MissingFixtureDefinition {
        fixture_id: String,
        definition_id: String,
    },
    MissingFixtureMode {
        fixture_id: String,
        definition_id: String,
        mode_id: String,
    },
    InvalidUniverse {
        fixture_id: String,
    },
    InvalidStartAddress {
        fixture_id: String,
    },
    AddressOutOfBounds {
        fixture_id: String,
        start_address: u16,
        channel_count: u16,
    },
    AddressCollision {
        first_fixture_id: String,
        second_fixture_id: String,
        universe: u16,
        start_address: u16,
        end_address: u16,
    },
}

#[derive(Clone, Debug)]
struct Assignment {
    fixture_id: String,
    universe: u16,
    start_address: u16,
    end_address: u16,
}

impl Patch {
    pub fn validate(&self) -> Result<(), Vec<PatchValidationError>> {
        let mut errors = Vec::new();
        let mut definitions_by_id = HashMap::new();

        for definition in &self.fixture_definitions {
            if definition.id.is_empty() {
                errors.push(PatchValidationError::EmptyFixtureDefinitionId);
                continue;
            }

            if definitions_by_id
                .insert(definition.id.as_str(), definition)
                .is_some()
            {
                errors.push(PatchValidationError::DuplicateFixtureDefinitionId {
                    definition_id: definition.id.clone(),
                });
            }

            let mut mode_ids = HashMap::new();
            for mode in &definition.modes {
                if mode.id.is_empty() {
                    errors.push(PatchValidationError::EmptyFixtureModeId {
                        definition_id: definition.id.clone(),
                    });
                    continue;
                }

                if mode_ids.insert(mode.id.as_str(), ()).is_some() {
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
            if fixture.id.is_empty() {
                errors.push(PatchValidationError::EmptyFixtureId);
            } else if fixture_ids.insert(fixture.id.as_str(), ()).is_some() {
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

            let Some(definition) = definitions_by_id.get(fixture.definition_id.as_str()) else {
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

    fn fixture_definition(channel_count: u16) -> FixtureDefinition {
        FixtureDefinition {
            id: "profile".to_owned(),
            manufacturer: "Iris".to_owned(),
            name: "Test fixture".to_owned(),
            modes: vec![FixtureMode {
                id: "standard".to_owned(),
                name: "Standard".to_owned(),
                channel_count,
            }],
        }
    }

    fn fixture(id: &str, number: u32, universe: u16, start_address: u16) -> FixtureInstance {
        FixtureInstance {
            id: id.to_owned(),
            number,
            name: format!("Fixture {number}"),
            definition_id: "profile".to_owned(),
            mode_id: "standard".to_owned(),
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
            fixture_id: "fixture-1".to_owned(),
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
                first_fixture_id: "fixture-1".to_owned(),
                second_fixture_id: "fixture-2".to_owned(),
                universe: 1,
                start_address: 4,
                end_address: 4,
            }])
        );
    }

    #[test]
    fn reports_missing_definition_and_mode() {
        let mut show_patch = patch(4, vec![fixture("fixture-1", 1, 1, 1)]);
        show_patch.fixtures[0].definition_id = "missing".to_owned();

        let errors = show_patch.validate().unwrap_err();
        assert!(
            errors.contains(&PatchValidationError::MissingFixtureDefinition {
                fixture_id: "fixture-1".to_owned(),
                definition_id: "missing".to_owned(),
            })
        );

        show_patch.fixtures[0].definition_id = "profile".to_owned();
        show_patch.fixtures[0].mode_id = "missing".to_owned();
        let errors = show_patch.validate().unwrap_err();
        assert!(errors.contains(&PatchValidationError::MissingFixtureMode {
            fixture_id: "fixture-1".to_owned(),
            definition_id: "profile".to_owned(),
            mode_id: "missing".to_owned(),
        }));
    }

    #[test]
    fn rejects_invalid_numbers_universes_and_zero_channel_modes() {
        let mut show_patch = patch(0, vec![fixture("fixture-1", 0, 0, 0)]);
        let errors = show_patch.validate().unwrap_err();

        assert!(errors.contains(&PatchValidationError::InvalidChannelCount {
            definition_id: "profile".to_owned(),
            mode_id: "standard".to_owned(),
            channel_count: 0,
        }));
        assert!(
            errors.contains(&PatchValidationError::InvalidFixtureNumber {
                fixture_id: "fixture-1".to_owned(),
            })
        );
        assert!(errors.contains(&PatchValidationError::InvalidUniverse {
            fixture_id: "fixture-1".to_owned(),
        }));
        assert!(errors.contains(&PatchValidationError::InvalidStartAddress {
            fixture_id: "fixture-1".to_owned(),
        }));

        show_patch.fixtures[0].number = 1;
        assert!(show_patch.validate().is_err());
    }
}
