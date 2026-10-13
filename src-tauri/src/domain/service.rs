use std::{
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

use super::{
    ids::StableId,
    library::{RecoverySource, ShowLibrary},
    patch::{FixtureDefinition, FixtureInstance, Patch},
    persistence::{QuicksaveSummary, ShowSummary},
    session::{SessionSettings, ShowSession},
};

/// What the UI needs to know about the open show.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SessionStatus {
    pub id: StableId,
    pub name: String,
    pub dirty: bool,
    /// Describes the copy the show was recovered from, when opening had to recover it.
    pub recovered_from: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackupSummary {
    pub id: StableId,
    pub saved_at_ms: u64,
}

/// Application-level facade over the show library and the open session. It is
/// the only thing Tauri commands touch, and reports errors as display strings.
pub struct ShowService {
    library: ShowLibrary,
    settings: SessionSettings,
    session: Mutex<Option<ShowSession>>,
}

pub type ServiceResult<T> = Result<T, String>;

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

impl ShowService {
    pub fn new(root: impl Into<PathBuf>, app_version: &str) -> Self {
        let root = root.into();
        Self {
            library: ShowLibrary::new(root),
            settings: SessionSettings::new(app_version),
            session: Mutex::new(None),
        }
    }

    pub fn list_shows(&self) -> ServiceResult<Vec<ShowSummary>> {
        self.library
            .list_shows()
            .map(|listing| listing.items)
            .map_err(|e| e.to_string())
    }

    pub fn new_show(&self, name: &str, now_ms: u64) -> ServiceResult<SessionStatus> {
        let id = StableId::new(unique_id(name, now_ms)).map_err(|_| "invalid show name")?;
        let session = ShowSession::new_show(
            self.library.clone(),
            self.settings.clone(),
            id,
            name,
            now_ms,
        )
        .map_err(|e| e.to_string())?;
        Ok(self.install(session))
    }

    pub fn open_show(&self, id: &str, now_ms: u64) -> ServiceResult<SessionStatus> {
        let id = StableId::new(id).map_err(|_| "invalid show id")?;
        let session = ShowSession::open(self.library.clone(), self.settings.clone(), &id, now_ms)
            .map_err(|e| e.to_string())?;
        Ok(self.install(session))
    }

    pub fn status(&self) -> ServiceResult<Option<SessionStatus>> {
        Ok(self.lock()?.as_ref().map(status_of))
    }

    pub fn save(&self, now_ms: u64) -> ServiceResult<SessionStatus> {
        self.with_session(|s| s.save(now_ms).map_err(|e| e.to_string()))
    }

    pub fn save_as(&self, name: &str, now_ms: u64) -> ServiceResult<SessionStatus> {
        let id = StableId::new(unique_id(name, now_ms)).map_err(|_| "invalid show name")?;
        self.with_session(|s| s.save_as(id, name, now_ms).map_err(|e| e.to_string()))
    }

    pub fn rename(&self, name: &str) -> ServiceResult<SessionStatus> {
        self.with_session(|s| {
            s.edit(|show| show.name = name.to_owned())
                .map_err(join_errors)
        })
    }

    pub fn quicksave(&self, label: &str, now_ms: u64) -> ServiceResult<Vec<QuicksaveSummary>> {
        self.with_session(|s| s.quicksave(label, now_ms).map(drop).map_err(|e| e.to_string()))?;
        self.list_quicksaves()
    }

    pub fn list_quicksaves(&self) -> ServiceResult<Vec<QuicksaveSummary>> {
        self.read_session(|s| {
            s.library()
                .list_quicksaves(&s.show().id)
                .map(|listing| listing.items.iter().rev().map(|q| q.summary()).collect())
                .map_err(|e| e.to_string())
        })
    }

    pub fn restore_quicksave(&self, id: &str) -> ServiceResult<SessionStatus> {
        let id = StableId::new(id).map_err(|_| "invalid quicksave id")?;
        self.with_session(|s| s.restore_quicksave(&id).map_err(|e| e.to_string()))
    }

    pub fn delete_quicksave(&self, id: &str) -> ServiceResult<Vec<QuicksaveSummary>> {
        let id = StableId::new(id).map_err(|_| "invalid quicksave id")?;
        self.read_session(|s| s.delete_quicksave(&id).map_err(|e| e.to_string()))?;
        self.list_quicksaves()
    }

    pub fn list_backups(&self) -> ServiceResult<Vec<BackupSummary>> {
        self.read_session(|s| {
            s.library()
                .list_backups(&s.show().id)
                .map(|listing| {
                    listing
                        .items
                        .into_iter()
                        .map(|(id, file)| BackupSummary {
                            id,
                            saved_at_ms: file.saved_at_ms,
                        })
                        .collect()
                })
                .map_err(|e| e.to_string())
        })
    }

    pub fn restore_backup(&self, id: &str) -> ServiceResult<SessionStatus> {
        let id = StableId::new(id).map_err(|_| "invalid backup id")?;
        self.with_session(|s| s.restore_backup(&id).map_err(|e| e.to_string()))
    }

    pub fn get_patch(&self) -> ServiceResult<Patch> {
        self.read_session(|s| Ok(s.show().patch.clone()))
    }

    /// Adds or updates fixtures (matched by id) together with any definitions
    /// they need, as one atomic edit. Nothing changes if the result is invalid.
    pub fn save_fixtures(
        &self,
        definitions: Vec<FixtureDefinition>,
        fixtures: Vec<FixtureInstance>,
    ) -> ServiceResult<Patch> {
        self.edit_patch(move |patch| {
            for definition in definitions {
                match patch
                    .fixture_definitions
                    .iter_mut()
                    .find(|existing| existing.id == definition.id)
                {
                    Some(existing) => *existing = definition,
                    None => patch.fixture_definitions.push(definition),
                }
            }
            for fixture in fixtures {
                match patch.fixtures.iter_mut().find(|existing| existing.id == fixture.id) {
                    Some(existing) => *existing = fixture,
                    None => patch.fixtures.push(fixture),
                }
            }
        })
    }

    pub fn remove_fixtures(&self, ids: Vec<StableId>) -> ServiceResult<Patch> {
        self.edit_patch(move |patch| patch.fixtures.retain(|fixture| !ids.contains(&fixture.id)))
    }

    pub fn get_workspace(&self) -> ServiceResult<Option<serde_json::Value>> {
        self.read_session(|s| Ok(s.show().workspace.clone()))
    }

    pub fn set_workspace(&self, workspace: serde_json::Value) -> ServiceResult<SessionStatus> {
        self.with_session(|s| {
            s.edit(|show| show.workspace = Some(workspace))
                .map_err(join_errors)
        })
    }

    fn edit_patch(&self, change: impl FnOnce(&mut Patch)) -> ServiceResult<Patch> {
        let mut guard = self.lock()?;
        let session = guard.as_mut().ok_or("no show is open")?;
        session
            .edit(|show| change(&mut show.patch))
            .map_err(join_errors)?;
        Ok(session.show().patch.clone())
    }

    /// Called periodically by the app; returns true when an automatic quicksave was made.
    pub fn tick(&self, now_ms: u64) -> ServiceResult<bool> {
        let mut guard = self.lock()?;
        match guard.as_mut() {
            Some(session) => session
                .tick(now_ms)
                .map(|created| created.is_some())
                .map_err(|e| e.to_string()),
            None => Ok(false),
        }
    }

    fn install(&self, session: ShowSession) -> SessionStatus {
        let status = status_of(&session);
        // A poisoned lock only means an earlier command panicked; the slot is still replaceable.
        *self.session.lock().unwrap_or_else(|e| e.into_inner()) = Some(session);
        status
    }

    fn lock(&self) -> ServiceResult<std::sync::MutexGuard<'_, Option<ShowSession>>> {
        self.session
            .lock()
            .map_err(|_| "show session is unavailable".to_owned())
    }

    fn with_session(
        &self,
        action: impl FnOnce(&mut ShowSession) -> ServiceResult<()>,
    ) -> ServiceResult<SessionStatus> {
        let mut guard = self.lock()?;
        let session = guard.as_mut().ok_or("no show is open")?;
        action(session)?;
        Ok(status_of(session))
    }

    fn read_session<R>(
        &self,
        action: impl FnOnce(&ShowSession) -> ServiceResult<R>,
    ) -> ServiceResult<R> {
        let guard = self.lock()?;
        action(guard.as_ref().ok_or("no show is open")?)
    }
}

fn join_errors(errors: Vec<super::show::ShowValidationError>) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

fn status_of(session: &ShowSession) -> SessionStatus {
    SessionStatus {
        id: session.show().id.clone(),
        name: session.show().name.clone(),
        dirty: session.is_dirty(),
        recovered_from: session.recovery().map(|source| match source {
            RecoverySource::Backup(id) => format!("backup {id}"),
            RecoverySource::Quicksave(id) => format!("quicksave {id}"),
        }),
    }
}

/// Builds a file-safe id such as `opening-night-1700000000000`.
fn unique_id(name: &str, now_ms: u64) -> String {
    let slug: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = if slug.is_empty() { "show".to_owned() } else { slug };
    format!("{slug}-{now_ms}")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("iris-service-test-{}-{tag}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn commands_need_an_open_show() {
        let dir = TempDir::new("closed");
        let service = ShowService::new(&dir.0, "0.1.0");

        assert_eq!(service.status().unwrap(), None);
        assert_eq!(service.save(1).unwrap_err(), "no show is open");
        assert_eq!(service.list_quicksaves().unwrap_err(), "no show is open");
        assert!(!service.tick(1).unwrap());
    }

    #[test]
    fn creates_edits_saves_and_reopens_a_show() {
        let dir = TempDir::new("flow");
        let service = ShowService::new(&dir.0, "0.1.0");
        let created = service.new_show("Opening Night!", 1_000).unwrap();
        assert_eq!(created.id.as_str(), "opening-night-1000");

        assert!(service.rename("Closing Night").unwrap().dirty);
        assert!(!service.save(2_000).unwrap().dirty);

        let other = ShowService::new(&dir.0, "0.1.0");
        let shows = other.list_shows().unwrap();
        assert_eq!(shows.len(), 1);
        let opened = other.open_show(shows[0].id.as_str(), 3_000).unwrap();
        assert_eq!(opened.name, "Closing Night");
        assert_eq!(opened.recovered_from, None);
    }

    #[test]
    fn invalid_edits_are_reported_as_messages() {
        let dir = TempDir::new("invalid");
        let service = ShowService::new(&dir.0, "0.1.0");
        service.new_show("Tour", 1).unwrap();

        assert_eq!(
            service.rename(" ").unwrap_err(),
            "show name must not be empty"
        );
        assert_eq!(service.status().unwrap().unwrap().name, "Tour");
    }

    #[test]
    fn quicksaves_and_backups_are_listed_newest_first() {
        let dir = TempDir::new("lists");
        let service = ShowService::new(&dir.0, "0.1.0");
        service.new_show("Tour", 1).unwrap();
        service.quicksave("first", 10).unwrap();
        let quicksaves = service.quicksave("second", 20).unwrap();
        let labels: Vec<_> = quicksaves.iter().map(|q| q.label.as_str()).collect();
        assert_eq!(labels, vec!["second", "first"]);

        service.rename("Tour 2").unwrap();
        service.save(30).unwrap();
        let backups = service.list_backups().unwrap();
        assert_eq!(backups.len(), 1);

        let restored = service.restore_quicksave(quicksaves[1].id.as_str()).unwrap();
        assert_eq!((restored.name.as_str(), restored.dirty), ("Tour", true));
        let restored = service.restore_backup(backups[0].id.as_str()).unwrap();
        assert_eq!(restored.name, "Tour");

        let left = service.delete_quicksave(quicksaves[0].id.as_str()).unwrap();
        assert_eq!(left.len(), 1);
    }

    #[test]
    fn tick_creates_automatic_quicksaves_only_for_unsaved_changes() {
        let dir = TempDir::new("tick");
        let service = ShowService::new(&dir.0, "0.1.0");
        service.new_show("Tour", 0).unwrap();

        assert!(!service.tick(u64::MAX / 2).unwrap());
        service.rename("Tour 2").unwrap();
        assert!(service.tick(u64::MAX / 2).unwrap());
        assert_eq!(service.list_quicksaves().unwrap().len(), 1);
    }

    fn definition() -> FixtureDefinition {
        FixtureDefinition {
            id: StableId::new("dimmer").unwrap(),
            manufacturer: "Generic".into(),
            name: "Dimmer".into(),
            modes: vec![super::super::patch::FixtureMode {
                id: StableId::new("default").unwrap(),
                name: "Default".into(),
                channel_count: 4,
            }],
        }
    }

    fn fixture(id: &str, number: u32, address: u16) -> FixtureInstance {
        FixtureInstance {
            id: StableId::new(id).unwrap(),
            number,
            name: id.into(),
            definition_id: StableId::new("dimmer").unwrap(),
            mode_id: StableId::new("default").unwrap(),
            universe: 1,
            start_address: address,
        }
    }

    #[test]
    fn patch_edits_are_validated_atomically_and_persist_with_the_show() {
        let dir = TempDir::new("patch");
        let service = ShowService::new(&dir.0, "0.1.0");
        let status = service.new_show("Tour", 1).unwrap();

        let patch = service
            .save_fixtures(vec![definition()], vec![fixture("a", 1, 1), fixture("b", 2, 5)])
            .unwrap();
        assert_eq!(patch.fixtures.len(), 2);

        let error = service
            .save_fixtures(vec![], vec![fixture("c", 3, 3)])
            .unwrap_err();
        assert!(error.contains("overlap"), "{error}");
        assert_eq!(service.get_patch().unwrap().fixtures.len(), 2);

        let moved = service.save_fixtures(vec![], vec![fixture("b", 2, 9)]).unwrap();
        assert_eq!(moved.fixtures[1].start_address, 9);

        service.save(2).unwrap();
        let other = ShowService::new(&dir.0, "0.1.0");
        other.open_show(status.id.as_str(), 3).unwrap();
        assert_eq!(other.get_patch().unwrap().fixtures.len(), 2);

        let left = other
            .remove_fixtures(vec![StableId::new("a").unwrap()])
            .unwrap();
        assert_eq!(left.fixtures.len(), 1);
    }

    #[test]
    fn workspace_layout_is_stored_in_the_show() {
        let dir = TempDir::new("workspace");
        let service = ShowService::new(&dir.0, "0.1.0");
        let status = service.new_show("Tour", 1).unwrap();
        assert_eq!(service.get_workspace().unwrap(), None);

        let layout = serde_json::json!({ "views": [{ "windows": [] }], "activeView": 0 });
        assert!(service.set_workspace(layout.clone()).unwrap().dirty);
        service.save(2).unwrap();

        let other = ShowService::new(&dir.0, "0.1.0");
        other.open_show(status.id.as_str(), 3).unwrap();
        assert_eq!(other.get_workspace().unwrap(), Some(layout));
    }

    #[test]
    fn unique_ids_are_file_safe() {
        assert_eq!(unique_id("  ../My Show  ", 5), "my-show-5");
        assert_eq!(unique_id("***", 5), "show-5");
    }
}
