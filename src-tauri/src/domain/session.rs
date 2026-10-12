use super::{
    ids::StableId,
    library::{LoadOutcome, RecoverySource, ShowLibrary},
    persistence::{PersistenceError, QuicksaveKind, ShowFile},
    show::{Show, ShowValidationError},
};

pub const DEFAULT_AUTO_QUICKSAVE_INTERVAL_MS: u64 = 5 * 60 * 1000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionSettings {
    pub app_version: String,
    pub auto_quicksave_interval_ms: u64,
    pub max_auto_quicksaves: usize,
    pub max_backups: usize,
}

impl SessionSettings {
    pub fn new(app_version: impl Into<String>) -> Self {
        Self {
            app_version: app_version.into(),
            auto_quicksave_interval_ms: DEFAULT_AUTO_QUICKSAVE_INTERVAL_MS,
            max_auto_quicksaves: super::persistence::DEFAULT_MAX_QUICKSAVES,
            max_backups: super::library::DEFAULT_MAX_BACKUPS,
        }
    }
}

/// The show currently open in the console, with save state. All time comes from
/// the caller, so behavior is deterministic and independent of the UI.
#[derive(Debug)]
pub struct ShowSession {
    library: ShowLibrary,
    settings: SessionSettings,
    current: ShowFile,
    dirty: bool,
    last_auto_quicksave_ms: u64,
    quicksave_counter: u64,
    recovery: Option<RecoverySource>,
}

impl ShowSession {
    pub fn new_show(
        library: ShowLibrary,
        settings: SessionSettings,
        id: StableId,
        name: &str,
        now_ms: u64,
    ) -> Result<Self, PersistenceError> {
        let current = library.create_show(id, name, &settings.app_version, now_ms)?;
        Ok(Self::from_file(library, settings, current, now_ms))
    }

    pub fn open(
        library: ShowLibrary,
        settings: SessionSettings,
        id: &StableId,
        now_ms: u64,
    ) -> Result<Self, PersistenceError> {
        let (current, recovery) = match library.load_or_recover(id)? {
            LoadOutcome::Loaded(file) => (file, None),
            LoadOutcome::Recovered { file, source, .. } => (file, Some(source)),
        };
        let mut session = Self::from_file(library, settings, current, now_ms);
        session.recovery = recovery;
        Ok(session)
    }

    fn from_file(
        library: ShowLibrary,
        settings: SessionSettings,
        current: ShowFile,
        now_ms: u64,
    ) -> Self {
        Self {
            library,
            settings,
            current,
            dirty: false,
            last_auto_quicksave_ms: now_ms,
            quicksave_counter: 0,
            recovery: None,
        }
    }

    pub fn show(&self) -> &Show {
        &self.current.show
    }

    /// Set when opening had to recover the show from a backup or quicksave.
    pub fn recovery(&self) -> Option<&RecoverySource> {
        self.recovery.as_ref()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn library(&self) -> &ShowLibrary {
        &self.library
    }

    /// Applies an edit to a copy of the show and keeps it only if the result is
    /// valid, so an invalid edit never reaches the open show.
    pub fn edit<R>(
        &mut self,
        change: impl FnOnce(&mut Show) -> R,
    ) -> Result<R, Vec<ShowValidationError>> {
        let mut candidate = self.current.show.clone();
        let result = change(&mut candidate);
        candidate.validate()?;
        self.current.show = candidate;
        self.dirty = true;
        Ok(result)
    }

    pub fn save(&mut self, now_ms: u64) -> Result<(), PersistenceError> {
        let mut file = self.current.clone();
        file.saved_at_ms = now_ms;
        self.library
            .backup_existing(&file.show.id, self.settings.max_backups)?;
        self.library.save_show(&file)?;
        self.current = file;
        self.dirty = false;
        Ok(())
    }

    /// Saves a copy under a new identity and switches the session to it.
    pub fn save_as(
        &mut self,
        new_id: StableId,
        new_name: &str,
        now_ms: u64,
    ) -> Result<(), PersistenceError> {
        self.current = self
            .library
            .save_show_as(&self.current, new_id, new_name, now_ms)?;
        self.dirty = false;
        self.last_auto_quicksave_ms = now_ms;
        Ok(())
    }

    /// Snapshots the open show, including unsaved edits. Returns the new quicksave id.
    pub fn quicksave(&mut self, label: &str, now_ms: u64) -> Result<StableId, PersistenceError> {
        self.write_quicksave(QuicksaveKind::Manual, label, now_ms)
    }

    /// Creates an automatic quicksave when the interval has elapsed and there
    /// are unsaved changes. Returns the id when one was created.
    pub fn tick(&mut self, now_ms: u64) -> Result<Option<StableId>, PersistenceError> {
        let due = now_ms.saturating_sub(self.last_auto_quicksave_ms)
            >= self.settings.auto_quicksave_interval_ms;
        if !self.dirty || !due {
            return Ok(None);
        }
        self.write_quicksave(QuicksaveKind::Auto, "Auto", now_ms)
            .map(Some)
    }

    /// Replaces the open show with a quicksave's snapshot. The result counts as
    /// unsaved, so the saved show file is untouched until the operator saves.
    pub fn restore_quicksave(&mut self, quicksave_id: &StableId) -> Result<(), PersistenceError> {
        let snapshot = self
            .library
            .restore_quicksave(&self.current.show.id, quicksave_id)?;
        self.current = snapshot;
        self.dirty = true;
        Ok(())
    }

    /// Replaces the open show with a backup's contents; counts as unsaved.
    pub fn restore_backup(&mut self, backup_id: &StableId) -> Result<(), PersistenceError> {
        let snapshot = self
            .library
            .restore_backup(&self.current.show.id, backup_id)?;
        self.current = snapshot;
        self.dirty = true;
        Ok(())
    }

    pub fn delete_quicksave(&self, quicksave_id: &StableId) -> Result<(), PersistenceError> {
        self.library
            .delete_quicksave(&self.current.show.id, quicksave_id)
    }

    fn write_quicksave(
        &mut self,
        kind: QuicksaveKind,
        label: &str,
        now_ms: u64,
    ) -> Result<StableId, PersistenceError> {
        self.quicksave_counter += 1;
        let prefix = match kind {
            QuicksaveKind::Manual => "quicksave",
            QuicksaveKind::Auto => "auto",
        };
        let id = StableId::new(format!("{prefix}-{now_ms}-{}", self.quicksave_counter))
            .expect("generated id is not blank");

        let mut snapshot = self.current.clone();
        snapshot.saved_at_ms = now_ms;
        self.library.quicksave_show(
            &snapshot,
            id.clone(),
            kind,
            label,
            now_ms,
            self.settings.max_auto_quicksaves,
        )?;
        if kind == QuicksaveKind::Auto {
            self.last_auto_quicksave_ms = now_ms;
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("iris-session-test-{}-{tag}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn id(value: &str) -> StableId {
        StableId::new(value).unwrap()
    }

    fn session(dir: &TempDir) -> ShowSession {
        let mut settings = SessionSettings::new("0.1.0");
        settings.auto_quicksave_interval_ms = 100;
        settings.max_auto_quicksaves = 2;
        ShowSession::new_show(ShowLibrary::new(&dir.0), settings, id("s1"), "Tour", 0).unwrap()
    }

    fn rename(session: &mut ShowSession, name: &str) {
        session.edit(|show| show.name = name.to_owned()).unwrap();
    }

    #[test]
    fn edits_mark_the_show_dirty_until_saved() {
        let dir = TempDir::new("dirty");
        let mut session = session(&dir);
        assert!(!session.is_dirty());

        rename(&mut session, "Tour 2");
        assert!(session.is_dirty());

        session.save(50).unwrap();
        assert!(!session.is_dirty());
        let reopened = session.library().load_show(&id("s1")).unwrap();
        assert_eq!((reopened.show.name.as_str(), reopened.saved_at_ms), ("Tour 2", 50));
    }

    #[test]
    fn invalid_edits_are_rolled_back() {
        let dir = TempDir::new("rollback");
        let mut session = session(&dir);

        assert!(session.edit(|show| show.name = " ".to_owned()).is_err());
        assert_eq!(session.show().name, "Tour");
        assert!(!session.is_dirty());
    }

    #[test]
    fn auto_quicksave_needs_changes_and_an_elapsed_interval() {
        let dir = TempDir::new("auto");
        let mut session = session(&dir);

        assert_eq!(session.tick(500).unwrap(), None);
        rename(&mut session, "Tour 2");
        assert_eq!(session.tick(50).unwrap(), None);

        let created = session.tick(200).unwrap().expect("quicksave due");
        assert_eq!(session.tick(250).unwrap(), None);

        let stored = session.library().list_quicksaves(&id("s1")).unwrap().items;
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, created);
        assert_eq!(stored[0].kind, QuicksaveKind::Auto);
        assert_eq!(stored[0].file.show.name, "Tour 2");
    }

    #[test]
    fn auto_quicksaves_are_bounded_but_manual_ones_are_kept() {
        let dir = TempDir::new("bounded");
        let mut session = session(&dir);
        session.quicksave("keep", 1).unwrap();
        for step in 1..=4u64 {
            rename(&mut session, &format!("v{step}"));
            session.tick(step * 1_000).unwrap().unwrap();
        }

        let items = session.library().list_quicksaves(&id("s1")).unwrap().items;
        let manual = items.iter().filter(|q| q.kind == QuicksaveKind::Manual).count();
        assert_eq!((manual, items.len()), (1, 3));
    }

    #[test]
    fn restoring_a_quicksave_leaves_the_saved_file_untouched() {
        let dir = TempDir::new("restore");
        let mut session = session(&dir);
        let checkpoint = session.quicksave("before", 1).unwrap();

        rename(&mut session, "Changed");
        session.save(2).unwrap();
        session.restore_quicksave(&checkpoint).unwrap();

        assert_eq!(session.show().name, "Tour");
        assert!(session.is_dirty());
        assert_eq!(
            session.library().load_show(&id("s1")).unwrap().show.name,
            "Changed"
        );
    }

    #[test]
    fn save_as_switches_the_session_to_the_copy() {
        let dir = TempDir::new("saveas");
        let mut session = session(&dir);
        rename(&mut session, "Draft");
        session.save_as(id("s2"), "Final", 9).unwrap();

        assert_eq!((session.show().id.as_str(), session.show().name.as_str()), ("s2", "Final"));
        assert!(!session.is_dirty());
        session.quicksave("x", 10).unwrap();
        assert_eq!(session.library().list_quicksaves(&id("s2")).unwrap().items.len(), 1);
        assert!(session.library().list_quicksaves(&id("s1")).unwrap().items.is_empty());
    }

    #[test]
    fn saving_keeps_the_previous_version_as_a_backup() {
        let dir = TempDir::new("backup");
        let mut session = session(&dir);
        rename(&mut session, "Tour 2");
        session.save(50).unwrap();

        let backups = session.library().list_backups(&id("s1")).unwrap().items;
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].1.show.name, "Tour");
    }

    #[test]
    fn open_recovers_a_corrupt_show_and_reports_it() {
        let dir = TempDir::new("recover");
        let mut first = session(&dir);
        rename(&mut first, "Tour 2");
        first.save(50).unwrap();
        fs::write(first.library().layout().show_file(&id("s1")), b"{oops").unwrap();

        let reopened = ShowSession::open(
            ShowLibrary::new(&dir.0),
            SessionSettings::new("0.1.0"),
            &id("s1"),
            60,
        )
        .unwrap();
        assert_eq!(reopened.show().name, "Tour");
        assert!(matches!(reopened.recovery(), Some(RecoverySource::Backup(_))));
    }

    #[test]
    fn open_loads_a_saved_show_clean() {
        let dir = TempDir::new("open");
        drop(session(&dir));
        let session = ShowSession::open(
            ShowLibrary::new(&dir.0),
            SessionSettings::new("0.1.0"),
            &id("s1"),
            0,
        )
        .unwrap();

        assert_eq!(session.show().name, "Tour");
        assert!(!session.is_dirty());
    }
}
