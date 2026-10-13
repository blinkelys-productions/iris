//! Durable profile catalogue: archives on disk plus a JSON index.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::gdtf::{self, GdtfMeta};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Official,
    Community,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Record {
    pub id: String,
    pub fixture_type_id: String,
    pub manufacturer: String,
    pub name: String,
    pub long_name: String,
    pub revision: String,
    pub modes: Vec<String>,
    pub sha256: String,
    pub size: u64,
    pub source: Source,
    pub uploader: String,
    pub uploaded_at_ms: u64,
    pub downloads: u64,
    /// Lets a community uploader retract their file; never sent to clients.
    #[serde(default)]
    pub delete_token_hash: Option<String>,
}

/// What clients see: a record without secrets.
#[derive(Clone, Debug, Serialize)]
pub struct PublicRecord {
    pub id: String,
    pub fixture_type_id: String,
    pub manufacturer: String,
    pub name: String,
    pub long_name: String,
    pub revision: String,
    pub modes: Vec<String>,
    pub sha256: String,
    pub size: u64,
    pub source: Source,
    pub uploader: String,
    pub uploaded_at_ms: u64,
    pub downloads: u64,
}

impl From<&Record> for PublicRecord {
    fn from(r: &Record) -> Self {
        Self {
            id: r.id.clone(),
            fixture_type_id: r.fixture_type_id.clone(),
            manufacturer: r.manufacturer.clone(),
            name: r.name.clone(),
            long_name: r.long_name.clone(),
            revision: r.revision.clone(),
            modes: r.modes.clone(),
            sha256: r.sha256.clone(),
            size: r.size,
            source: r.source,
            uploader: r.uploader.clone(),
            uploaded_at_ms: r.uploaded_at_ms,
            downloads: r.downloads,
        }
    }
}

pub enum Uploaded {
    Created { record: PublicRecord, delete_token: Option<String> },
    Existing(PublicRecord),
}

#[derive(Debug)]
pub enum StoreError {
    Invalid(String),
    NotFound,
    Forbidden,
    Io(String),
}

pub struct Store {
    dir: PathBuf,
    records: Mutex<Vec<Record>>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn io(error: impl std::fmt::Display) -> StoreError {
    StoreError::Io(error.to_string())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let tmp = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp).map_err(io)?;
    file.write_all(bytes).map_err(io)?;
    file.sync_all().map_err(io)?;
    fs::rename(&tmp, path).map_err(io)
}

/// Trims, drops control characters and bounds the length of free-text fields.
fn clean(value: &str, max: usize) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(max)
        .collect()
}

impl Store {
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let dir = dir.into();
        fs::create_dir_all(dir.join("files")).map_err(io)?;
        let records = match fs::read(dir.join("index.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(io(e)),
        };
        Ok(Self { dir, records: Mutex::new(records) })
    }

    fn file_path(&self, id: &str) -> PathBuf {
        self.dir.join("files").join(format!("{id}.gdtf"))
    }

    fn persist(&self, records: &[Record]) -> Result<(), StoreError> {
        let json = serde_json::to_vec_pretty(records).map_err(io)?;
        write_atomic(&self.dir.join("index.json"), &json)
    }

    pub fn list(&self, query: &str, source: Option<Source>) -> Vec<PublicRecord> {
        let needle = query.trim().to_lowercase();
        let records = self.records.lock().unwrap();
        let mut found: Vec<&Record> = records
            .iter()
            .filter(|r| source.is_none_or(|s| r.source == s))
            .filter(|r| {
                needle.is_empty()
                    || [&r.manufacturer, &r.name, &r.long_name]
                        .iter()
                        .any(|field| field.to_lowercase().contains(&needle))
            })
            .collect();
        found.sort_by(|a, b| {
            let rank = |r: &Record| u8::from(r.source != Source::Official);
            (rank(a), a.manufacturer.to_lowercase(), a.name.to_lowercase())
                .cmp(&(rank(b), b.manufacturer.to_lowercase(), b.name.to_lowercase()))
        });
        found.into_iter().map(PublicRecord::from).collect()
    }

    pub fn get(&self, id: &str) -> Option<PublicRecord> {
        let records = self.records.lock().unwrap();
        records.iter().find(|r| r.id == id).map(PublicRecord::from)
    }

    /// Reads the archive and counts the download.
    pub fn download(&self, id: &str) -> Result<(PublicRecord, Vec<u8>), StoreError> {
        let mut records = self.records.lock().unwrap();
        let index = records.iter().position(|r| r.id == id).ok_or(StoreError::NotFound)?;
        let bytes = fs::read(self.file_path(id)).map_err(io)?;
        records[index].downloads += 1;
        let public = PublicRecord::from(&records[index]);
        // A lost counter update must not block the download.
        let _ = self.persist(&records);
        Ok((public, bytes))
    }

    pub fn upload(
        &self,
        bytes: &[u8],
        source: Source,
        uploader: &str,
    ) -> Result<Uploaded, StoreError> {
        let meta: GdtfMeta = gdtf::inspect(bytes).map_err(StoreError::Invalid)?;
        let sha = sha256_hex(bytes);
        let mut records = self.records.lock().unwrap();

        if let Some(existing) = records.iter_mut().find(|r| r.sha256 == sha) {
            // Re-uploading an identical community file as official upgrades it in place.
            if source == Source::Official && existing.source != Source::Official {
                existing.source = Source::Official;
                existing.delete_token_hash = None;
                let public = PublicRecord::from(&*existing);
                self.persist(&records)?;
                return Ok(Uploaded::Created { record: public, delete_token: None });
            }
            return Ok(Uploaded::Existing(PublicRecord::from(&*existing)));
        }

        let id = Uuid::new_v4().to_string();
        let (delete_token, delete_token_hash) = match source {
            Source::Community => {
                let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
                let hash = sha256_hex(token.as_bytes());
                (Some(token), Some(hash))
            }
            Source::Official => (None, None),
        };
        let uploader = clean(uploader, 64);
        let record = Record {
            id: id.clone(),
            fixture_type_id: meta.fixture_type_id,
            manufacturer: clean(&meta.manufacturer, 128),
            name: clean(&meta.name, 128),
            long_name: clean(&meta.long_name, 256),
            revision: clean(&meta.revision, 64),
            modes: meta.modes.iter().map(|m| clean(m, 128)).collect(),
            sha256: sha,
            size: bytes.len() as u64,
            source,
            uploader: if uploader.is_empty() { "anonymous".to_owned() } else { uploader },
            uploaded_at_ms: now_ms(),
            downloads: 0,
            delete_token_hash,
        };

        write_atomic(&self.file_path(&id), bytes)?;
        let public = PublicRecord::from(&record);
        records.push(record);
        if let Err(error) = self.persist(&records) {
            records.pop();
            let _ = fs::remove_file(self.file_path(&id));
            return Err(error);
        }
        Ok(Uploaded::Created { record: public, delete_token })
    }

    /// Removes a profile. `delete_token` of `None` means the caller is an administrator.
    pub fn delete(&self, id: &str, delete_token: Option<&str>) -> Result<(), StoreError> {
        let mut records = self.records.lock().unwrap();
        let index = records.iter().position(|r| r.id == id).ok_or(StoreError::NotFound)?;
        if let Some(token) = delete_token {
            let matches = records[index].source == Source::Community
                && records[index].delete_token_hash.as_deref() == Some(sha256_hex(token.as_bytes()).as_str());
            if !matches {
                return Err(StoreError::Forbidden);
            }
        }
        let removed = records.remove(index);
        if let Err(error) = self.persist(&records) {
            records.insert(index, removed);
            return Err(error);
        }
        let _ = fs::remove_file(self.file_path(id));
        Ok(())
    }

    pub fn set_source(&self, id: &str, source: Source) -> Result<PublicRecord, StoreError> {
        let mut records = self.records.lock().unwrap();
        let record = records.iter_mut().find(|r| r.id == id).ok_or(StoreError::NotFound)?;
        record.source = source;
        if source == Source::Official {
            record.delete_token_hash = None;
        }
        let public = PublicRecord::from(&*record);
        self.persist(&records)?;
        Ok(public)
    }

    pub fn stats(&self) -> (usize, usize) {
        let records = self.records.lock().unwrap();
        let official = records.iter().filter(|r| r.source == Source::Official).count();
        (official, records.len() - official)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gdtf::sample_archive;

    fn store() -> (Store, PathBuf) {
        let dir = std::env::temp_dir().join(format!("gdtf-share-{}", Uuid::new_v4()));
        (Store::open(&dir).unwrap(), dir)
    }

    #[test]
    fn upload_dedupes_lists_downloads_and_deletes() {
        let (store, dir) = store();
        let bytes = sample_archive("Spot");
        let Uploaded::Created { record, delete_token } =
            store.upload(&bytes, Source::Community, "ann").unwrap()
        else {
            panic!("expected created")
        };
        let token = delete_token.unwrap();
        assert!(matches!(store.upload(&bytes, Source::Community, "bob").unwrap(), Uploaded::Existing(_)));
        assert_eq!(store.list("spo", None).len(), 1);
        assert!(store.list("zzz", None).is_empty());

        assert_eq!(store.download(&record.id).unwrap().1, bytes);
        assert_eq!(store.get(&record.id).unwrap().downloads, 1);

        assert!(matches!(store.delete(&record.id, Some("wrong")), Err(StoreError::Forbidden)));
        store.delete(&record.id, Some(&token)).unwrap();
        assert!(store.get(&record.id).is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn official_upload_promotes_and_sorts_first_and_survives_reopen() {
        let (store, dir) = store();
        store.upload(&sample_archive("Alpha"), Source::Community, "").unwrap();
        let Uploaded::Created { record, .. } =
            store.upload(&sample_archive("Zed"), Source::Official, "iris").unwrap()
        else {
            panic!()
        };
        assert_eq!(store.list("", None)[0].name, "Zed");
        assert!(matches!(store.delete(&record.id, Some("x")), Err(StoreError::Forbidden)));

        let reopened = Store::open(&dir).unwrap();
        assert_eq!(reopened.stats(), (1, 1));
        let _ = fs::remove_dir_all(dir);
    }
}
