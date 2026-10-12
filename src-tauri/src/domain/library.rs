use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::{
    ids::StableId,
    persistence::{
        PersistenceError, Quicksave, QuicksaveKind, QuicksaveLog, SHOW_FILE_EXTENSION, ShowFile,
        ShowLibraryLayout, ShowSummary,
    },
    show::Show,
};

/// Items found in a directory plus the files that could not be read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Listing<T> {
    pub items: Vec<T>,
    pub unreadable: Vec<PathBuf>,
}

pub const DEFAULT_MAX_BACKUPS: usize = 10;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoverySource {
    Backup(StableId),
    Quicksave(StableId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoadOutcome {
    Loaded(ShowFile),
    /// The saved file was unreadable; it was moved aside and replaced by the newest valid copy.
    Recovered {
        file: ShowFile,
        source: RecoverySource,
        quarantined: PathBuf,
    },
}

/// File-backed show storage. All writes are atomic: data goes to a temporary
/// sibling file that is flushed and then renamed over the target.
#[derive(Clone, Debug)]
pub struct ShowLibrary {
    layout: ShowLibraryLayout,
}

impl ShowLibrary {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            layout: ShowLibraryLayout::new(root),
        }
    }

    pub fn layout(&self) -> &ShowLibraryLayout {
        &self.layout
    }

    /// Creates and saves an empty show; fails if the id is already in use.
    pub fn create_show(
        &self,
        id: StableId,
        name: &str,
        app_version: &str,
        now_ms: u64,
    ) -> Result<ShowFile, PersistenceError> {
        ensure_safe(&id)?;
        if self.layout.show_file(&id).exists() {
            return Err(PersistenceError::ShowAlreadyExists { id });
        }
        let file = ShowFile::new(Show::new(id, name), app_version, now_ms);
        self.save_show(&file)?;
        Ok(file)
    }

    pub fn save_show(&self, file: &ShowFile) -> Result<(), PersistenceError> {
        ensure_safe(&file.show.id)?;
        write_atomic(&self.layout.show_file(&file.show.id), &file.encode()?)
    }

    /// Saves a copy of `file` as a new show with its own id and name.
    pub fn save_show_as(
        &self,
        file: &ShowFile,
        new_id: StableId,
        new_name: &str,
        now_ms: u64,
    ) -> Result<ShowFile, PersistenceError> {
        ensure_safe(&new_id)?;
        if self.layout.show_file(&new_id).exists() {
            return Err(PersistenceError::ShowAlreadyExists { id: new_id });
        }
        let mut copy = file.clone();
        copy.show.id = new_id;
        copy.show.name = new_name.to_owned();
        copy.saved_at_ms = now_ms;
        self.save_show(&copy)?;
        Ok(copy)
    }

    pub fn load_show(&self, id: &StableId) -> Result<ShowFile, PersistenceError> {
        ensure_safe(id)?;
        let path = self.layout.show_file(id);
        match read(&path) {
            Ok(bytes) => ShowFile::decode(&bytes),
            Err(PersistenceError::Io { .. }) if !path.exists() => {
                Err(PersistenceError::ShowNotFound { id: id.clone() })
            }
            Err(error) => Err(error),
        }
    }

    /// Lists saved shows, most recently saved first.
    pub fn list_shows(&self) -> Result<Listing<ShowSummary>, PersistenceError> {
        let mut listing = Listing {
            items: Vec::new(),
            unreadable: Vec::new(),
        };
        for dir in sub_entries(&self.layout.root)? {
            let path = dir.join(super::persistence::SHOW_FILE_NAME);
            if !path.is_file() {
                continue;
            }
            match read(&path).and_then(|bytes| ShowFile::decode(&bytes)) {
                Ok(file) => listing.items.push(file.summary()),
                Err(_) => listing.unreadable.push(path),
            }
        }
        listing
            .items
            .sort_by(|a, b| b.saved_at_ms.cmp(&a.saved_at_ms).then(a.id.cmp(&b.id)));
        Ok(listing)
    }

    pub fn delete_show(&self, id: &StableId) -> Result<(), PersistenceError> {
        ensure_safe(id)?;
        let dir = self.layout.show_dir(id);
        if !dir.exists() {
            return Err(PersistenceError::ShowNotFound { id: id.clone() });
        }
        fs::remove_dir_all(&dir).map_err(|error| io_error(&dir, error))
    }

    /// Writes a quicksave and removes automatic quicksaves beyond the retention
    /// limit. Returns the ids that were evicted.
    pub fn create_quicksave(
        &self,
        show_id: &StableId,
        quicksave: Quicksave,
        max_auto: usize,
    ) -> Result<Vec<StableId>, PersistenceError> {
        ensure_safe(show_id)?;
        ensure_safe(&quicksave.id)?;

        let mut log = QuicksaveLog::new(max_auto);
        for existing in self.list_quicksaves(show_id)?.items {
            log.push(existing)?;
        }
        let evicted = log.push(quicksave.clone())?;

        let bytes = serde_json::to_vec_pretty(&quicksave)
            .map_err(|error| PersistenceError::Encode(error.to_string()))?;
        write_atomic(&self.layout.quicksave_file(show_id, &quicksave.id), &bytes)?;
        for id in &evicted {
            let path = self.layout.quicksave_file(show_id, id);
            fs::remove_file(&path).map_err(|error| io_error(&path, error))?;
        }
        Ok(evicted)
    }

    /// Lists quicksaves oldest first.
    pub fn list_quicksaves(
        &self,
        show_id: &StableId,
    ) -> Result<Listing<Quicksave>, PersistenceError> {
        ensure_safe(show_id)?;
        let mut listing = Listing {
            items: Vec::new(),
            unreadable: Vec::new(),
        };
        let dir = self.layout.quicksave_dir(show_id);
        if !dir.exists() {
            return Ok(listing);
        }
        for path in files_in(&dir)? {
            if path.extension().and_then(|e| e.to_str()) != Some(SHOW_FILE_EXTENSION) {
                continue;
            }
            match read_quicksave(&path) {
                Ok(quicksave) => listing.items.push(quicksave),
                Err(_) => listing.unreadable.push(path),
            }
        }
        listing
            .items
            .sort_by(|a, b| a.created_at_ms.cmp(&b.created_at_ms).then(a.id.cmp(&b.id)));
        Ok(listing)
    }

    /// Returns the snapshot stored in a quicksave; the caller decides when to make it current.
    pub fn restore_quicksave(
        &self,
        show_id: &StableId,
        quicksave_id: &StableId,
    ) -> Result<ShowFile, PersistenceError> {
        ensure_safe(show_id)?;
        ensure_safe(quicksave_id)?;
        let path = self.layout.quicksave_file(show_id, quicksave_id);
        if !path.exists() {
            return Err(PersistenceError::QuicksaveNotFound {
                id: quicksave_id.clone(),
            });
        }
        Ok(read_quicksave(&path)?.file)
    }

    pub fn delete_quicksave(
        &self,
        show_id: &StableId,
        quicksave_id: &StableId,
    ) -> Result<(), PersistenceError> {
        ensure_safe(show_id)?;
        ensure_safe(quicksave_id)?;
        let path = self.layout.quicksave_file(show_id, quicksave_id);
        fs::remove_file(&path).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                PersistenceError::QuicksaveNotFound {
                    id: quicksave_id.clone(),
                }
            } else {
                io_error(&path, error)
            }
        })
    }

    /// Copies the show file currently on disk into `backups/` before it is
    /// overwritten, then prunes the oldest backups beyond `max_backups`.
    /// Does nothing when there is no valid saved file to protect.
    pub fn backup_existing(
        &self,
        show_id: &StableId,
        max_backups: usize,
    ) -> Result<Option<StableId>, PersistenceError> {
        ensure_safe(show_id)?;
        let Ok(bytes) = read(&self.layout.show_file(show_id)) else {
            return Ok(None);
        };
        let Ok(existing) = ShowFile::decode(&bytes) else {
            return Ok(None);
        };

        let backup_id = StableId::new(format!("backup-{}", existing.saved_at_ms))
            .expect("generated id is not blank");
        write_atomic(&self.backup_file(show_id, &backup_id), &bytes)?;

        let mut backups = self.list_backups(show_id)?.items;
        backups.sort_by(|a, b| a.1.saved_at_ms.cmp(&b.1.saved_at_ms).then(a.0.cmp(&b.0)));
        while backups.len() > max_backups {
            let (old_id, _) = backups.remove(0);
            let path = self.backup_file(show_id, &old_id);
            fs::remove_file(&path).map_err(|error| io_error(&path, error))?;
        }
        Ok(Some(backup_id))
    }

    /// Lists backups as `(id, file)` pairs, newest first.
    pub fn list_backups(
        &self,
        show_id: &StableId,
    ) -> Result<Listing<(StableId, ShowFile)>, PersistenceError> {
        ensure_safe(show_id)?;
        let mut listing = Listing {
            items: Vec::new(),
            unreadable: Vec::new(),
        };
        let dir = self.layout.backup_dir(show_id);
        if !dir.exists() {
            return Ok(listing);
        }
        for path in files_in(&dir)? {
            if path.extension().and_then(|e| e.to_str()) != Some(SHOW_FILE_EXTENSION) {
                continue;
            }
            let id = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| StableId::new(stem).ok());
            match (id, read(&path).and_then(|bytes| ShowFile::decode(&bytes))) {
                (Some(id), Ok(file)) => listing.items.push((id, file)),
                _ => listing.unreadable.push(path),
            }
        }
        listing
            .items
            .sort_by(|a, b| b.1.saved_at_ms.cmp(&a.1.saved_at_ms).then(a.0.cmp(&b.0)));
        Ok(listing)
    }

    pub fn restore_backup(
        &self,
        show_id: &StableId,
        backup_id: &StableId,
    ) -> Result<ShowFile, PersistenceError> {
        ensure_safe(show_id)?;
        ensure_safe(backup_id)?;
        let path = self.backup_file(show_id, backup_id);
        if !path.exists() {
            return Err(PersistenceError::BackupNotFound {
                id: backup_id.clone(),
            });
        }
        ShowFile::decode(&read(&path)?)
    }

    pub fn delete_backup(
        &self,
        show_id: &StableId,
        backup_id: &StableId,
    ) -> Result<(), PersistenceError> {
        ensure_safe(show_id)?;
        ensure_safe(backup_id)?;
        let path = self.backup_file(show_id, backup_id);
        fs::remove_file(&path).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                PersistenceError::BackupNotFound {
                    id: backup_id.clone(),
                }
            } else {
                io_error(&path, error)
            }
        })
    }

    /// Loads a show, and if its file is corrupt, moves it aside to
    /// `show.iris.corrupt` and restores the most recently saved valid backup or
    /// quicksave. If nothing valid exists the original error is returned and
    /// the file is left where it is.
    pub fn load_or_recover(&self, show_id: &StableId) -> Result<LoadOutcome, PersistenceError> {
        let original = match self.load_show(show_id) {
            Ok(file) => return Ok(LoadOutcome::Loaded(file)),
            Err(error @ PersistenceError::ShowNotFound { .. }) => return Err(error),
            Err(error) => error,
        };

        let mut candidates: Vec<(ShowFile, RecoverySource)> = Vec::new();
        for (id, file) in self.list_backups(show_id)?.items {
            candidates.push((file, RecoverySource::Backup(id)));
        }
        for quicksave in self.list_quicksaves(show_id)?.items {
            candidates.push((quicksave.file, RecoverySource::Quicksave(quicksave.id)));
        }
        let Some((file, source)) = candidates
            .into_iter()
            .max_by_key(|(file, _)| file.saved_at_ms)
        else {
            return Err(original);
        };

        let path = self.layout.show_file(show_id);
        let quarantined = path.with_extension("iris.corrupt");
        fs::rename(&path, &quarantined).map_err(|error| io_error(&path, error))?;
        self.save_show(&file)?;
        Ok(LoadOutcome::Recovered {
            file,
            source,
            quarantined,
        })
    }

    fn backup_file(&self, show_id: &StableId, backup_id: &StableId) -> PathBuf {
        self.layout
            .backup_dir(show_id)
            .join(format!("{}.{SHOW_FILE_EXTENSION}", backup_id.as_str()))
    }

    /// Convenience for callers that snapshot the current show.
    pub fn quicksave_show(
        &self,
        file: &ShowFile,
        quicksave_id: StableId,
        kind: QuicksaveKind,
        label: &str,
        now_ms: u64,
        max_auto: usize,
    ) -> Result<Vec<StableId>, PersistenceError> {
        let quicksave = Quicksave {
            id: quicksave_id,
            kind,
            label: label.to_owned(),
            created_at_ms: now_ms,
            file: file.clone(),
        };
        self.create_quicksave(&file.show.id, quicksave, max_auto)
    }
}

/// Ids become directory and file names, so anything that could escape the library is refused.
fn ensure_safe(id: &StableId) -> Result<(), PersistenceError> {
    let value = id.as_str();
    let safe = value != "."
        && value != ".."
        && !value.starts_with('.')
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' '));
    if safe {
        Ok(())
    } else {
        Err(PersistenceError::UnsafeId {
            id: value.to_owned(),
        })
    }
}

fn io_error(path: &Path, error: io::Error) -> PersistenceError {
    PersistenceError::Io {
        path: path.display().to_string(),
        message: error.to_string(),
    }
}

fn read(path: &Path) -> Result<Vec<u8>, PersistenceError> {
    fs::read(path).map_err(|error| io_error(path, error))
}

fn read_quicksave(path: &Path) -> Result<Quicksave, PersistenceError> {
    let quicksave: Quicksave = serde_json::from_slice(&read(path)?)
        .map_err(|error| PersistenceError::Decode(error.to_string()))?;
    quicksave.file.validate()?;
    Ok(quicksave)
}

fn sub_entries(root: &Path) -> Result<Vec<PathBuf>, PersistenceError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut dirs = Vec::new();
    for entry in fs::read_dir(root).map_err(|error| io_error(root, error))? {
        let path = entry.map_err(|error| io_error(root, error))?.path();
        if path.is_dir() {
            dirs.push(path);
        }
    }
    Ok(dirs)
}

fn files_in(dir: &Path) -> Result<Vec<PathBuf>, PersistenceError> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).map_err(|error| io_error(dir, error))? {
        let path = entry.map_err(|error| io_error(dir, error))?.path();
        if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), PersistenceError> {
    use io::Write;

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;

    let mut temp_name = path.file_name().unwrap_or_default().to_os_string();
    temp_name.push(".tmp");
    let temp = parent.join(temp_name);

    let result = (|| {
        let mut file = fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)
    })();

    result.map_err(|error| {
        let _ = fs::remove_file(&temp);
        io_error(path, error)
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "iris-library-test-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            ));
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

    fn library() -> (TempDir, ShowLibrary) {
        let dir = TempDir::new();
        let library = ShowLibrary::new(&dir.0);
        (dir, library)
    }

    #[test]
    fn creates_saves_and_loads_a_show() {
        let (_dir, library) = library();
        let mut file = library.create_show(id("s1"), "Tour", "0.1.0", 10).unwrap();

        assert_eq!(library.load_show(&id("s1")).unwrap(), file);

        file.show.name = "Tour v2".to_owned();
        file.saved_at_ms = 20;
        library.save_show(&file).unwrap();
        assert_eq!(library.load_show(&id("s1")).unwrap().show.name, "Tour v2");

        let dir = library.layout().show_dir(&id("s1"));
        assert!(!dir.join("show.iris.tmp").exists());
    }

    #[test]
    fn refuses_duplicates_and_reports_missing_shows() {
        let (_dir, library) = library();
        library.create_show(id("s1"), "Tour", "0.1.0", 1).unwrap();

        assert!(matches!(
            library.create_show(id("s1"), "Again", "0.1.0", 2),
            Err(PersistenceError::ShowAlreadyExists { .. })
        ));
        assert!(matches!(
            library.load_show(&id("nope")),
            Err(PersistenceError::ShowNotFound { .. })
        ));
    }

    #[test]
    fn invalid_shows_are_not_written() {
        let (_dir, library) = library();
        assert!(library.create_show(id("s1"), " ", "0.1.0", 1).is_err());
        assert!(!library.layout().show_file(&id("s1")).exists());
    }

    #[test]
    fn rejects_ids_that_could_escape_the_library() {
        let (_dir, library) = library();
        for bad in ["../evil", "a/b", "..", ".hidden", "a\\b"] {
            assert!(matches!(
                library.create_show(id(bad), "X", "0.1.0", 1),
                Err(PersistenceError::UnsafeId { .. })
            ));
        }
    }

    #[test]
    fn save_as_copies_under_a_new_identity() {
        let (_dir, library) = library();
        let original = library.create_show(id("s1"), "Tour", "0.1.0", 1).unwrap();
        let copy = library
            .save_show_as(&original, id("s2"), "Tour copy", 5)
            .unwrap();

        assert_eq!(copy.show.name, "Tour copy");
        assert_eq!(library.load_show(&id("s1")).unwrap().show.name, "Tour");
        assert!(matches!(
            library.save_show_as(&original, id("s2"), "Dup", 6),
            Err(PersistenceError::ShowAlreadyExists { .. })
        ));
    }

    #[test]
    fn lists_newest_first_and_flags_corrupt_shows() {
        let (_dir, library) = library();
        library.create_show(id("old"), "Old", "0.1.0", 1).unwrap();
        library.create_show(id("new"), "New", "0.1.0", 9).unwrap();
        let broken = library.layout().show_file(&id("broken"));
        fs::create_dir_all(broken.parent().unwrap()).unwrap();
        fs::write(&broken, b"{oops").unwrap();

        let listing = library.list_shows().unwrap();
        let names: Vec<_> = listing.items.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["New", "Old"]);
        assert_eq!(listing.unreadable, vec![broken]);
    }

    #[test]
    fn listing_an_empty_library_is_not_an_error() {
        let (_dir, library) = library();
        assert!(library.list_shows().unwrap().items.is_empty());
    }

    #[test]
    fn deletes_shows() {
        let (_dir, library) = library();
        library.create_show(id("s1"), "Tour", "0.1.0", 1).unwrap();
        library.delete_show(&id("s1")).unwrap();

        assert!(matches!(
            library.delete_show(&id("s1")),
            Err(PersistenceError::ShowNotFound { .. })
        ));
    }

    #[test]
    fn quicksaves_restore_the_snapshot_not_the_current_show() {
        let (_dir, library) = library();
        let mut file = library.create_show(id("s1"), "Before", "0.1.0", 1).unwrap();
        library
            .quicksave_show(&file, id("q1"), QuicksaveKind::Manual, "pre-show", 2, 5)
            .unwrap();

        file.show.name = "After".to_owned();
        library.save_show(&file).unwrap();

        let restored = library.restore_quicksave(&id("s1"), &id("q1")).unwrap();
        assert_eq!(restored.show.name, "Before");
        assert_eq!(library.load_show(&id("s1")).unwrap().show.name, "After");
    }

    #[test]
    fn automatic_quicksaves_are_pruned_on_disk() {
        let (_dir, library) = library();
        let file = library.create_show(id("s1"), "Tour", "0.1.0", 1).unwrap();
        library
            .quicksave_show(&file, id("m1"), QuicksaveKind::Manual, "keep", 1, 2)
            .unwrap();
        for (n, name) in ["a1", "a2", "a3"].into_iter().enumerate() {
            let evicted = library
                .quicksave_show(&file, id(name), QuicksaveKind::Auto, name, 10 + n as u64, 2)
                .unwrap();
            assert_eq!(evicted.len(), usize::from(n == 2));
        }

        let ids: Vec<_> = library
            .list_quicksaves(&id("s1"))
            .unwrap()
            .items
            .into_iter()
            .map(|q| q.id.to_string())
            .collect();
        assert_eq!(ids, vec!["m1", "a2", "a3"]);
    }

    #[test]
    fn quicksave_errors_are_specific() {
        let (_dir, library) = library();
        let file = library.create_show(id("s1"), "Tour", "0.1.0", 1).unwrap();
        library
            .quicksave_show(&file, id("q1"), QuicksaveKind::Manual, "x", 1, 5)
            .unwrap();

        assert!(matches!(
            library.quicksave_show(&file, id("q1"), QuicksaveKind::Manual, "x", 2, 5),
            Err(PersistenceError::DuplicateQuicksave { .. })
        ));
        assert!(matches!(
            library.restore_quicksave(&id("s1"), &id("zz")),
            Err(PersistenceError::QuicksaveNotFound { .. })
        ));

        library.delete_quicksave(&id("s1"), &id("q1")).unwrap();
        assert!(matches!(
            library.delete_quicksave(&id("s1"), &id("q1")),
            Err(PersistenceError::QuicksaveNotFound { .. })
        ));
        assert!(library.list_quicksaves(&id("s1")).unwrap().items.is_empty());
    }

    #[test]
    fn saving_over_a_show_keeps_a_bounded_set_of_backups() {
        let (_dir, library) = library();
        let mut file = library.create_show(id("s1"), "v0", "0.1.0", 1).unwrap();
        assert_eq!(library.backup_existing(&id("nope"), 3).unwrap(), None);

        for step in 2..=6u64 {
            library.backup_existing(&id("s1"), 3).unwrap();
            file.show.name = format!("v{step}");
            file.saved_at_ms = step;
            library.save_show(&file).unwrap();
        }

        let backups = library.list_backups(&id("s1")).unwrap().items;
        let times: Vec<_> = backups.iter().map(|(_, f)| f.saved_at_ms).collect();
        assert_eq!(times, vec![5, 4, 3]);
        let restored = library.restore_backup(&id("s1"), &backups[0].0).unwrap();
        assert_eq!(restored.show.name, "v5");
    }

    #[test]
    fn backup_errors_are_specific() {
        let (_dir, library) = library();
        library.create_show(id("s1"), "Tour", "0.1.0", 1).unwrap();
        let backup = library.backup_existing(&id("s1"), 3).unwrap().unwrap();

        library.delete_backup(&id("s1"), &backup).unwrap();
        assert!(matches!(
            library.delete_backup(&id("s1"), &backup),
            Err(PersistenceError::BackupNotFound { .. })
        ));
        assert!(matches!(
            library.restore_backup(&id("s1"), &backup),
            Err(PersistenceError::BackupNotFound { .. })
        ));
    }

    #[test]
    fn a_corrupt_show_file_is_not_backed_up() {
        let (_dir, library) = library();
        library.create_show(id("s1"), "Tour", "0.1.0", 1).unwrap();
        fs::write(library.layout().show_file(&id("s1")), b"{oops").unwrap();

        assert_eq!(library.backup_existing(&id("s1"), 3).unwrap(), None);
        assert!(library.list_backups(&id("s1")).unwrap().items.is_empty());
    }

    #[test]
    fn recovers_from_the_newest_valid_copy_and_quarantines_the_corrupt_file() {
        let (_dir, library) = library();
        let mut file = library.create_show(id("s1"), "v1", "0.1.0", 1).unwrap();
        library.backup_existing(&id("s1"), 5).unwrap();
        file.show.name = "v2".to_owned();
        file.saved_at_ms = 5;
        library
            .quicksave_show(&file, id("q1"), QuicksaveKind::Manual, "newer", 5, 5)
            .unwrap();
        fs::write(library.layout().show_file(&id("s1")), b"{oops").unwrap();

        let LoadOutcome::Recovered { file, source, quarantined } =
            library.load_or_recover(&id("s1")).unwrap()
        else {
            panic!("expected recovery");
        };
        assert_eq!(file.show.name, "v2");
        assert_eq!(source, RecoverySource::Quicksave(id("q1")));
        assert_eq!(fs::read(&quarantined).unwrap(), b"{oops");
        assert_eq!(library.load_show(&id("s1")).unwrap().show.name, "v2");
        assert!(matches!(
            library.load_or_recover(&id("s1")).unwrap(),
            LoadOutcome::Loaded(_)
        ));
    }

    #[test]
    fn recovery_without_any_valid_copy_leaves_the_file_alone() {
        let (_dir, library) = library();
        library.create_show(id("s1"), "Tour", "0.1.0", 1).unwrap();
        let path = library.layout().show_file(&id("s1"));
        fs::write(&path, b"{oops").unwrap();

        assert!(matches!(
            library.load_or_recover(&id("s1")),
            Err(PersistenceError::Decode(_))
        ));
        assert_eq!(fs::read(&path).unwrap(), b"{oops");
        assert!(matches!(
            library.load_or_recover(&id("missing")),
            Err(PersistenceError::ShowNotFound { .. })
        ));
    }
}
