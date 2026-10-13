//! Console-wide store of imported GDTF fixture profiles.
//!
//! Each profile is kept as the original archive (needed for MVR export) next to a
//! JSON copy of its definition, so listing never has to re-parse GDTF files.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use uuid::Uuid;

use super::{
    ids::StableId,
    interchange::ImportedGdtf,
    library::{io_error, write_atomic},
    patch::FixtureDefinition,
    persistence::PersistenceError,
};

#[derive(Clone, Debug)]
pub struct ProfileStore {
    dir: PathBuf,
}

impl ProfileStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn save(&self, imported: &ImportedGdtf) -> Result<(), PersistenceError> {
        let json = serde_json::to_vec_pretty(&imported.definition)
            .map_err(|error| PersistenceError::Encode(error.to_string()))?;
        write_atomic(&self.archive_path(&imported.definition.id), &imported.archive)?;
        write_atomic(&self.definition_path(&imported.definition.id), &json)
    }

    /// Every readable profile, sorted by manufacturer then name. Damaged entries are skipped.
    pub fn list(&self) -> Result<Vec<FixtureDefinition>, PersistenceError> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(io_error(&self.dir, error)),
        };
        let mut definitions: Vec<FixtureDefinition> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .filter_map(|path| fs::read(path).ok())
            .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
            .collect();
        definitions.sort_by(|a, b| {
            (&a.manufacturer, &a.name, &a.id).cmp(&(&b.manufacturer, &b.name, &b.id))
        });
        Ok(definitions)
    }

    /// Archives for the requested definitions; ids without a stored archive are omitted.
    pub fn archives(&self, ids: &[StableId]) -> BTreeMap<StableId, Vec<u8>> {
        ids.iter()
            .filter_map(|id| {
                fs::read(self.archive_path(id))
                    .ok()
                    .map(|bytes| (id.clone(), bytes))
            })
            .collect()
    }

    fn archive_path(&self, id: &StableId) -> PathBuf {
        self.dir.join(format!("{}.gdtf", file_stem(id)))
    }

    fn definition_path(&self, id: &StableId) -> PathBuf {
        self.dir.join(format!("{}.json", file_stem(id)))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

/// Definition ids come from imported files, so file names are derived rather than trusted.
fn file_stem(id: &StableId) -> String {
    Uuid::new_v5(&Uuid::NAMESPACE_OID, id.as_str().as_bytes()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::patch::FixtureMode;

    fn imported(id: &str, name: &str) -> ImportedGdtf {
        ImportedGdtf {
            definition: FixtureDefinition {
                id: StableId::new(id).unwrap(),
                manufacturer: "Acme".to_owned(),
                name: name.to_owned(),
                modes: vec![FixtureMode {
                    id: StableId::new("m1").unwrap(),
                    name: "Standard".to_owned(),
                    channel_count: 8,
                }],
            },
            archive_file_name: format!("{name}.gdtf"),
            archive: name.as_bytes().to_vec(),
        }
    }

    #[test]
    fn saves_lists_and_returns_archives_even_for_unsafe_ids() {
        let dir = std::env::temp_dir().join(format!("iris-profiles-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let store = ProfileStore::new(&dir);

        assert!(store.list().unwrap().is_empty());
        store.save(&imported("../evil/id", "Zed")).unwrap();
        store.save(&imported("b", "Alpha")).unwrap();

        let names: Vec<_> = store.list().unwrap().into_iter().map(|d| d.name).collect();
        assert_eq!(names, ["Alpha", "Zed"]);

        let evil = StableId::new("../evil/id").unwrap();
        let archives = store.archives(&[evil.clone(), StableId::new("missing").unwrap()]);
        assert_eq!(archives.len(), 1);
        assert_eq!(archives[&evil], b"Zed");
        assert!(!dir.parent().unwrap().join("evil").exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
