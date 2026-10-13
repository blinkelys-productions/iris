//! Client for an Iris GDTF Share server.

use std::{io::Read, time::Duration};

use serde::{Deserialize, Serialize};

const MAX_DOWNLOAD_BYTES: u64 = 40 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SharedProfile {
    pub id: String,
    pub manufacturer: String,
    pub name: String,
    pub revision: String,
    pub modes: Vec<String>,
    pub source: String,
    pub uploader: String,
    pub downloads: u64,
    pub size: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShareUpload {
    pub profile: SharedProfile,
    pub delete_token: Option<String>,
    pub duplicate: bool,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: String,
}

pub struct ShareClient {
    base: String,
    agent: ureq::Agent,
}

impl ShareClient {
    pub fn new(base_url: &str) -> Result<Self, String> {
        let base = base_url.trim().trim_end_matches('/').to_owned();
        if !(base.starts_with("http://") || base.starts_with("https://")) || base.len() < 9 {
            return Err("the share server address must start with http:// or https://".to_owned());
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .http_status_as_error(false)
            .build()
            .into();
        Ok(Self { base, agent })
    }

    fn check(&self, mut response: ureq::http::Response<ureq::Body>) -> Result<ureq::http::Response<ureq::Body>, String> {
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status();
        let message = response
            .body_mut()
            .read_json::<ErrorBody>()
            .map(|body| body.error)
            .unwrap_or_else(|_| status.to_string());
        Err(format!("share server: {message}"))
    }

    fn unreachable(&self, error: ureq::Error) -> String {
        format!("cannot reach the share server at {}: {error}", self.base)
    }

    pub fn search(&self, query: &str, source: &str) -> Result<Vec<SharedProfile>, String> {
        let response = self
            .agent
            .get(format!("{}/api/v1/profiles", self.base))
            .query("q", query)
            .query("source", source)
            .call()
            .map_err(|e| self.unreachable(e))?;
        self.check(response)?
            .body_mut()
            .read_json()
            .map_err(|e| format!("unexpected share server reply: {e}"))
    }

    pub fn download(&self, id: &str) -> Result<Vec<u8>, String> {
        let response = self
            .agent
            .get(format!("{}/api/v1/profiles/{}/download", self.base, encode_segment(id)))
            .call()
            .map_err(|e| self.unreachable(e))?;
        let mut response = self.check(response)?;
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(MAX_DOWNLOAD_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("download failed: {e}"))?;
        if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
            return Err("downloaded file is too large".to_owned());
        }
        Ok(bytes)
    }

    pub fn upload(&self, archive: &[u8], uploader: &str) -> Result<ShareUpload, String> {
        let uploader: String = uploader.chars().filter(|c| c.is_ascii_graphic() || *c == ' ').take(64).collect();
        let response = self
            .agent
            .post(format!("{}/api/v1/profiles", self.base))
            .header("X-Uploader", uploader.trim())
            .header("Content-Type", "application/octet-stream")
            .send(archive)
            .map_err(|e| self.unreachable(e))?;
        self.check(response)?
            .body_mut()
            .read_json()
            .map_err(|e| format!("unexpected share server reply: {e}"))
    }

    pub fn delete(&self, id: &str, token: &str) -> Result<(), String> {
        let response = self
            .agent
            .delete(format!("{}/api/v1/profiles/{}", self.base, encode_segment(id)))
            .header("X-Delete-Token", token)
            .call()
            .map_err(|e| self.unreachable(e))?;
        self.check(response).map(|_| ())
    }
}

/// Ids are server-issued UUIDs, but the path segment is escaped regardless.
fn encode_segment(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_urls_and_escapes_segments() {
        assert!(ShareClient::new("ftp://x").is_err());
        assert!(ShareClient::new("").is_err());
        assert!(ShareClient::new("http://localhost:8787/").is_ok());
        assert_eq!(encode_segment("a/../b"), "a%2F..%2Fb");
    }

    fn serve_once(status: &str, content_type: &str, body: &str) -> String {
        use std::io::Write;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let reply = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0u8; 4096];
            let _ = stream.read(&mut buffer);
            stream.write_all(reply.as_bytes()).unwrap();
        });
        format!("http://{address}")
    }

    #[test]
    fn parses_search_results_and_server_errors() {
        let body = r#"[{"id":"1","manufacturer":"Acme","name":"Spot","revision":"","modes":["A"],"source":"official","uploader":"Iris","downloads":2,"size":10,"sha256":"x"}]"#;
        let found = ShareClient::new(&serve_once("200 OK", "application/json", body)).unwrap().search("", "").unwrap();
        assert_eq!(found[0].name, "Spot");

        let error = ShareClient::new(&serve_once("422 Unprocessable Entity", "application/json", r#"{"error":"bad file"}"#))
            .unwrap()
            .upload(b"x", "me")
            .unwrap_err();
        assert!(error.contains("bad file"), "{error}");
    }
}
