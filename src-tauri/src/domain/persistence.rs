use std::{fmt, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::{
    ids::StableId,
    show::{Show, ShowValidationError},
};

/// Marker written into every Iris show file so other JSON is rejected early.
pub const SHOW_FILE_FORMAT: &str = "iris-show";
pub const SHOW_FILE_EXTENSION: &str = "iris";
pub const SHOW_FILE_NAME: &str = "show.iris";
pub const DEFAULT_MAX_QUICKSAVES: usize = 20;

/// One upgrade step, rewriting a whole show file (envelope included) in place.
pub type Migration = fn(&mut serde_json::Value) -> Result<(), String>;

/// `MIGRATIONS[n]` upgrades a file from format version `n + 1` to `n + 2`. Add an
/// entry here whenever `CURRENT_SHOW_FORMAT_VERSION` is raised.
pub const MIGRATIONS: &[Migration] = &[];

/// Upgrades an older show file to `current`. Files already at or beyond `current`
/// are left alone, so validation can report them.
pub fn migrate(
    file: &mut serde_json::Value,
    migrations: &[Migration],
    current: u32,
) -> Result<(), PersistenceError> {
    let version_of = |value: &serde_json::Value| {
        value
            .get("format_version")
            .and_then(serde_json::Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
    };
    // Foreign files without a version are rejected later with a clearer message.
    let Some(mut version) = version_of(file) else {
        return Ok(());
    };
    if file.get("format").and_then(serde_json::Value::as_str) != Some(SHOW_FILE_FORMAT) {
        return Ok(());
    }
    if version == 0 {
        return Err(PersistenceError::Decode("format version 0 is not valid".to_owned()));
    }
    while version < current {
        let step = migrations.get(version as usize - 1).ok_or_else(|| {
            PersistenceError::Decode(format!("no migration from format version {version}"))
        })?;
        step(file).map_err(|message| {
            PersistenceError::Decode(format!("migrating from version {version} failed: {message}"))
        })?;
        version += 1;
        file["format_version"] = version.into();
        if let Some(show) = file.get_mut("show") {
            show["format_version"] = version.into();
        }
    }
    Ok(())
}

/// The on-disk wrapper around a [`Show`]. Timestamps are supplied by the caller
/// (milliseconds since the Unix epoch) so the model stays deterministic.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShowFile {
    pub format: String,
    pub format_version: u32,
    pub app_version: String,
    pub saved_at_ms: u64,
    pub show: Show,
}

impl ShowFile {
    pub fn new(show: Show, app_version: impl Into<String>, saved_at_ms: u64) -> Self {
        Self {
            format: SHOW_FILE_FORMAT.to_owned(),
            format_version: show.format_version,
            app_version: app_version.into(),
            saved_at_ms,
            show,
        }
    }

    pub fn summary(&self) -> ShowSummary {
        ShowSummary {
            id: self.show.id.clone(),
            name: self.show.name.clone(),
            app_version: self.app_version.clone(),
            saved_at_ms: self.saved_at_ms,
        }
    }

    pub fn validate(&self) -> Result<(), PersistenceError> {
        if self.format != SHOW_FILE_FORMAT {
            return Err(PersistenceError::NotAShowFile {
                found: self.format.clone(),
            });
        }
        if self.format_version != self.show.format_version {
            return Err(PersistenceError::VersionMismatch {
                envelope: self.format_version,
                show: self.show.format_version,
            });
        }
        self.show.validate().map_err(PersistenceError::InvalidShow)
    }

    pub fn encode(&self) -> Result<Vec<u8>, PersistenceError> {
        self.validate()?;
        serde_json::to_vec_pretty(self).map_err(|error| PersistenceError::Encode(error.to_string()))
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, PersistenceError> {
        let mut value: serde_json::Value = serde_json::from_slice(bytes)
            .map_err(|error| PersistenceError::Decode(error.to_string()))?;
        migrate(&mut value, MIGRATIONS, super::show::CURRENT_SHOW_FORMAT_VERSION)?;
        let file: Self = serde_json::from_value(value)
            .map_err(|error| PersistenceError::Decode(error.to_string()))?;
        file.validate()?;
        Ok(file)
    }
}

/// Lightweight listing entry for a show, used by load and recent-show lists.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShowSummary {
    pub id: StableId,
    pub name: String,
    pub app_version: String,
    pub saved_at_ms: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum QuicksaveKind {
    /// Created by the operator; never evicted automatically.
    Manual,
    /// Created by the console on a timer or before risky operations.
    Auto,
}

/// A point-in-time snapshot of a show, restorable independently of the main file.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Quicksave {
    pub id: StableId,
    pub kind: QuicksaveKind,
    pub label: String,
    pub created_at_ms: u64,
    pub file: ShowFile,
}

/// Metadata-only view of a quicksave so lists need not carry whole shows.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QuicksaveSummary {
    pub id: StableId,
    pub kind: QuicksaveKind,
    pub label: String,
    pub created_at_ms: u64,
}

impl Quicksave {
    pub fn summary(&self) -> QuicksaveSummary {
        QuicksaveSummary {
            id: self.id.clone(),
            kind: self.kind,
            label: self.label.clone(),
            created_at_ms: self.created_at_ms,
        }
    }
}

/// Ordered quicksave history (oldest first) with bounded automatic retention.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QuicksaveLog {
    pub max_auto: usize,
    entries: Vec<Quicksave>,
}

impl Default for QuicksaveLog {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_QUICKSAVES)
    }
}

impl QuicksaveLog {
    pub fn new(max_auto: usize) -> Self {
        Self {
            max_auto,
            entries: Vec::new(),
        }
    }

    pub fn entries(&self) -> &[Quicksave] {
        &self.entries
    }

    pub fn get(&self, id: &StableId) -> Option<&Quicksave> {
        self.entries.iter().find(|entry| &entry.id == id)
    }

    /// Adds a quicksave and returns the ids of automatic quicksaves evicted to respect the limit.
    pub fn push(&mut self, quicksave: Quicksave) -> Result<Vec<StableId>, PersistenceError> {
        if self.get(&quicksave.id).is_some() {
            return Err(PersistenceError::DuplicateQuicksave {
                id: quicksave.id.clone(),
            });
        }
        quicksave.file.validate()?;
        self.entries.push(quicksave);

        let mut evicted = Vec::new();
        while self.auto_count() > self.max_auto {
            let oldest = self
                .entries
                .iter()
                .position(|entry| entry.kind == QuicksaveKind::Auto)
                .expect("auto_count > 0 implies an automatic entry exists");
            evicted.push(self.entries.remove(oldest).id);
        }
        Ok(evicted)
    }

    pub fn remove(&mut self, id: &StableId) -> Result<Quicksave, PersistenceError> {
        let index = self
            .entries
            .iter()
            .position(|entry| &entry.id == id)
            .ok_or_else(|| PersistenceError::QuicksaveNotFound { id: id.clone() })?;
        Ok(self.entries.remove(index))
    }

    pub fn summaries(&self) -> Vec<QuicksaveSummary> {
        self.entries.iter().map(Quicksave::summary).collect()
    }

    fn auto_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.kind == QuicksaveKind::Auto)
            .count()
    }
}

/// Directory layout for one show library. Pure path arithmetic; no I/O.
///
/// ```text
/// <root>/<show-id>/show.iris
/// <root>/<show-id>/quicksaves/<quicksave-id>.iris
/// <root>/<show-id>/backups/<name>.iris
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShowLibraryLayout {
    pub root: PathBuf,
}

impl ShowLibraryLayout {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn show_dir(&self, show_id: &StableId) -> PathBuf {
        self.root.join(show_id.as_str())
    }

    pub fn show_file(&self, show_id: &StableId) -> PathBuf {
        self.show_dir(show_id).join(SHOW_FILE_NAME)
    }

    pub fn quicksave_dir(&self, show_id: &StableId) -> PathBuf {
        self.show_dir(show_id).join("quicksaves")
    }

    pub fn quicksave_file(&self, show_id: &StableId, quicksave_id: &StableId) -> PathBuf {
        self.quicksave_dir(show_id)
            .join(format!("{}.{SHOW_FILE_EXTENSION}", quicksave_id.as_str()))
    }

    pub fn backup_dir(&self, show_id: &StableId) -> PathBuf {
        self.show_dir(show_id).join("backups")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PersistenceError {
    NotAShowFile { found: String },
    VersionMismatch { envelope: u32, show: u32 },
    InvalidShow(Vec<ShowValidationError>),
    Encode(String),
    Decode(String),
    DuplicateQuicksave { id: StableId },
    QuicksaveNotFound { id: StableId },
    BackupNotFound { id: StableId },
    ShowNotFound { id: StableId },
    ShowAlreadyExists { id: StableId },
    UnsafeId { id: String },
    Io { path: String, message: String },
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAShowFile { found } => {
                write!(formatter, "not an Iris show file (format '{found}')")
            }
            Self::VersionMismatch { envelope, show } => write!(
                formatter,
                "show file version {envelope} does not match its contents (version {show})"
            ),
            Self::InvalidShow(errors) => {
                write!(formatter, "show file contains an invalid show")?;
                for (index, error) in errors.iter().enumerate() {
                    write!(formatter, "{}{error}", if index == 0 { ": " } else { "; " })?;
                }
                Ok(())
            }
            Self::Encode(message) => write!(formatter, "could not encode show file: {message}"),
            Self::Decode(message) => write!(formatter, "could not read show file: {message}"),
            Self::DuplicateQuicksave { id } => write!(formatter, "quicksave '{id}' already exists"),
            Self::QuicksaveNotFound { id } => write!(formatter, "quicksave '{id}' was not found"),
            Self::BackupNotFound { id } => write!(formatter, "backup '{id}' was not found"),
            Self::ShowNotFound { id } => write!(formatter, "show '{id}' was not found"),
            Self::ShowAlreadyExists { id } => write!(formatter, "show '{id}' already exists"),
            Self::UnsafeId { id } => {
                write!(formatter, "identifier '{id}' cannot be used as a file name")
            }
            Self::Io { path, message } => write!(formatter, "{path}: {message}"),
        }
    }
}

impl std::error::Error for PersistenceError {}

#[cfg(test)]
mod tests {
    #[test]
    fn migrates_old_files_step_by_step_and_leaves_current_ones() {
        fn rename_title(file: &mut serde_json::Value) -> Result<(), String> {
            let title = file["show"]["title"].take();
            file["show"]["name"] = title;
            Ok(())
        }
        fn fail(_: &mut serde_json::Value) -> Result<(), String> {
            Err("boom".to_owned())
        }
        let old = serde_json::json!({
            "format": SHOW_FILE_FORMAT,
            "format_version": 1,
            "show": { "format_version": 1, "title": "Old" }
        });

        let mut migrated = old.clone();
        migrate(&mut migrated, &[rename_title], 2).unwrap();
        assert_eq!(migrated["format_version"], 2);
        assert_eq!(migrated["show"]["format_version"], 2);
        assert_eq!(migrated["show"]["name"], "Old");

        let mut current = old.clone();
        migrate(&mut current, &[rename_title], 1).unwrap();
        assert_eq!(current, old);

        assert!(migrate(&mut old.clone(), &[], 2).is_err());
        assert!(migrate(&mut old.clone(), &[fail], 2).is_err());
    }

    use super::*;

    fn id(value: &str) -> StableId {
        StableId::new(value).unwrap()
    }

    fn file(name: &str, saved_at_ms: u64) -> ShowFile {
        ShowFile::new(Show::new(id("show-1"), name), "0.1.0", saved_at_ms)
    }

    fn quicksave(name: &str, kind: QuicksaveKind, at: u64) -> Quicksave {
        Quicksave {
            id: id(name),
            kind,
            label: name.to_owned(),
            created_at_ms: at,
            file: file("Show", at),
        }
    }

    #[test]
    fn show_file_round_trips_through_bytes() {
        let original = file("Opening night", 1_000);
        let restored = ShowFile::decode(&original.encode().unwrap()).unwrap();

        assert_eq!(restored, original);
        assert_eq!(restored.summary().name, "Opening night");
    }

    #[test]
    fn decode_rejects_foreign_and_corrupt_files() {
        let mut foreign = file("Show", 1);
        foreign.format = "other".to_owned();
        let bytes = serde_json::to_vec(&foreign).unwrap();

        assert!(matches!(
            ShowFile::decode(&bytes),
            Err(PersistenceError::NotAShowFile { .. })
        ));
        assert!(matches!(
            ShowFile::decode(b"{not json"),
            Err(PersistenceError::Decode(_))
        ));
    }

    #[test]
    fn decode_rejects_unsupported_versions_and_invalid_shows() {
        let mut future = file("Show", 1);
        future.show.format_version += 1;
        future.format_version += 1;
        let bytes = serde_json::to_vec(&future).unwrap();
        assert!(matches!(
            ShowFile::decode(&bytes),
            Err(PersistenceError::InvalidShow(_))
        ));

        let mut mismatched = file("Show", 1);
        mismatched.format_version += 1;
        assert!(matches!(
            mismatched.validate(),
            Err(PersistenceError::VersionMismatch { .. })
        ));

        assert!(file(" ", 1).encode().is_err());
    }

    #[test]
    fn quicksave_log_evicts_only_the_oldest_automatic_entries() {
        let mut log = QuicksaveLog::new(2);
        log.push(quicksave("manual-1", QuicksaveKind::Manual, 1)).unwrap();
        log.push(quicksave("auto-1", QuicksaveKind::Auto, 2)).unwrap();
        log.push(quicksave("auto-2", QuicksaveKind::Auto, 3)).unwrap();
        let evicted = log.push(quicksave("auto-3", QuicksaveKind::Auto, 4)).unwrap();

        assert_eq!(evicted, vec![id("auto-1")]);
        let ids: Vec<_> = log.summaries().into_iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![id("manual-1"), id("auto-2"), id("auto-3")]);
    }

    #[test]
    fn quicksave_log_rejects_duplicates_and_reports_missing_entries() {
        let mut log = QuicksaveLog::default();
        log.push(quicksave("q1", QuicksaveKind::Manual, 1)).unwrap();

        assert!(matches!(
            log.push(quicksave("q1", QuicksaveKind::Manual, 2)),
            Err(PersistenceError::DuplicateQuicksave { .. })
        ));
        assert_eq!(log.remove(&id("q1")).unwrap().label, "q1");
        assert!(matches!(
            log.remove(&id("q1")),
            Err(PersistenceError::QuicksaveNotFound { .. })
        ));
    }

    #[test]
    fn layout_places_files_under_the_show_directory() {
        let layout = ShowLibraryLayout::new("/shows");

        assert_eq!(layout.show_file(&id("a")), PathBuf::from("/shows/a/show.iris"));
        assert_eq!(
            layout.quicksave_file(&id("a"), &id("q1")),
            PathBuf::from("/shows/a/quicksaves/q1.iris")
        );
        assert_eq!(layout.backup_dir(&id("a")), PathBuf::from("/shows/a/backups"));
    }
}
