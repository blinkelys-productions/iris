use std::collections::{BTreeMap, BTreeSet};

use super::ids::StableId;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeState {
    pub selected_fixture_ids: BTreeSet<StableId>,
    pub active_playback_ids: BTreeSet<StableId>,
    pub universe_output_states: BTreeMap<u16, UniverseOutputState>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UniverseOutputState {
    Disabled,
    Running,
    Faulted { message: String },
}
