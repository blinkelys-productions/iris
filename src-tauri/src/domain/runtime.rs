use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeState {
    pub selected_fixture_ids: BTreeSet<String>,
    pub active_playback_ids: BTreeSet<String>,
    pub universe_output_states: BTreeMap<u16, UniverseOutputState>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UniverseOutputState {
    Disabled,
    Running,
    Faulted { message: String },
}
