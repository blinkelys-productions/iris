use serde::{Deserialize, Serialize};

use super::{ids::StableId, patch::FixtureInstance};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CommandEnvelope {
    pub command_id: StableId,
    pub command: DomainCommand,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DomainCommand {
    PatchFixture {
        fixture: FixtureInstance,
    },
    MoveFixture {
        fixture_id: StableId,
        universe: u16,
        start_address: u16,
    },
    UnpatchFixture {
        fixture_id: StableId,
    },
    SelectFixtures {
        fixture_ids: Vec<StableId>,
    },
    ClearSelection,
    GoPlayback {
        playback_id: StableId,
    },
    ReleasePlayback {
        playback_id: StableId,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventEnvelope {
    pub event_id: StableId,
    pub caused_by: StableId,
    pub revision: u64,
    pub event: DomainEvent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DomainEvent {
    FixturePatched {
        fixture_id: StableId,
    },
    FixtureMoved {
        fixture_id: StableId,
        universe: u16,
        start_address: u16,
    },
    FixtureUnpatched {
        fixture_id: StableId,
    },
    SelectionChanged {
        fixture_ids: Vec<StableId>,
    },
    PlaybackStarted {
        playback_id: StableId,
    },
    PlaybackReleased {
        playback_id: StableId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_envelopes_round_trip_with_explicit_types() {
        let envelope = CommandEnvelope {
            command_id: StableId::new("command-1").unwrap(),
            command: DomainCommand::MoveFixture {
                fixture_id: StableId::new("fixture-1").unwrap(),
                universe: 2,
                start_address: 17,
            },
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let restored: CommandEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, envelope);
    }

    #[test]
    fn event_envelopes_link_back_to_the_command_and_revision() {
        let envelope = EventEnvelope {
            event_id: StableId::new("event-1").unwrap(),
            caused_by: StableId::new("command-1").unwrap(),
            revision: 4,
            event: DomainEvent::FixturePatched {
                fixture_id: StableId::new("fixture-1").unwrap(),
            },
        };

        assert_eq!(envelope.caused_by.as_str(), "command-1");
        assert_eq!(envelope.revision, 4);
    }
}
