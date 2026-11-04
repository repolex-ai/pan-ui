//! The one door to `pand`, Pan's daemon.
//!
//! pand holds every store on the machine and answers for each by its short
//! id. pan-ui reads from it (store list, SPARQL, search, thumbnails, full images,
//! facts) and writes exactly three properties back: `pan:rating`,
//! `pan:isPicked` and `pan:isRejected`. Every other route pand has — deliver,
//! delete, redo, rename-namespace, set membership — is out of reach because
//! no function here calls it.

use serde_json::{Value, json};
use std::time::Duration;

pub const DAEMON_PORT: u16 = 7401;
pub const PAN: &str = "https://repolex.ai/ontology/pan/";
pub const IMAGE_PREFIX: &str = "https://repolex.ai/pan/Image/";
pub const SET_PREFIX: &str = "https://repolex.ai/pan/ImageSet/";

#[derive(Clone)]
pub struct Pand {
    http: reqwest::Client,
    base: String,
}

/// One SPARQL result row: each variable's value, by position, or None when
/// the variable is unbound in that row.
pub type Row = Vec<Option<String>>;

impl Pand {
    pub fn new(port: u16) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(120))
            .build()
            .expect("http client");
        Pand { http, base: format!("http://127.0.0.1:{port}") }
    }

    pub fn url(&self) -> &str {
        &self.base
    }

    pub async fn get(&self, path: &str) -> Result<Value, String> {
        let r = self.http.get(format!("{}{path}", self.base)).send().await.map_err(|e| self.down(e))?;
        let status = r.status();
        if !status.is_success() {
            return Err(format!("pand {path}: {status}: {}", r.text().await.unwrap_or_default()));
        }
        r.json().await.map_err(|e| format!("pand {path}: {e}"))
    }

    /// Bytes and content type, for thumbnails and full images.
    pub async fn bytes(&self, path: &str) -> Result<(Vec<u8>, String), (u16, String)> {
        let r = self
            .http
            .get(format!("{}{path}", self.base))
            .send()
            .await
            .map_err(|e| (502, self.down(e)))?;
        let status = r.status();
        if !status.is_success() {
            return Err((status.as_u16(), r.text().await.unwrap_or_default()));
        }
        let ct = r
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        let b = r.bytes().await.map_err(|e| (502, e.to_string()))?;
        Ok((b.to_vec(), ct))
    }

    /// A SELECT, with each binding whole (type, value, datatype): for when a
    /// value has to be sent back to pand exactly as it came.
    pub async fn select_terms(&self, store: &str, query: &str) -> Result<Vec<Value>, String> {
        let r = self
            .http
            .post(format!("{}/stores/{store}/sparql", self.base))
            .header("Content-Type", "application/sparql-query")
            .header("Accept", "application/sparql-results+json")
            .body(query.to_string())
            .send()
            .await
            .map_err(|e| self.down(e))?;
        let status = r.status();
        if !status.is_success() {
            return Err(format!("pand sparql: {status}: {}", r.text().await.unwrap_or_default()));
        }
        let v: Value = r.json().await.map_err(|e| format!("pand sparql: {e}"))?;
        v.pointer("/results/bindings")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| "pand sparql: reply has no results.bindings".into())
    }

    /// A SELECT against one store. A reply with no `results.bindings` is an
    /// error, never an empty answer: "pand could not run it" and "nothing
    /// matched" must not look the same.
    pub async fn select(&self, store: &str, vars: &[&str], query: &str) -> Result<Vec<Row>, String> {
        let r = self
            .http
            .post(format!("{}/stores/{store}/sparql", self.base))
            .header("Content-Type", "application/sparql-query")
            .header("Accept", "application/sparql-results+json")
            .body(query.to_string())
            .send()
            .await
            .map_err(|e| self.down(e))?;
        let status = r.status();
        if !status.is_success() {
            return Err(format!("pand sparql: {status}: {}", r.text().await.unwrap_or_default()));
        }
        let v: Value = r.json().await.map_err(|e| format!("pand sparql: {e}"))?;
        let Some(rows) = v.pointer("/results/bindings").and_then(Value::as_array) else {
            return Err("pand sparql: reply has no results.bindings".into());
        };
        Ok(rows
            .iter()
            .map(|b| {
                vars.iter()
                    .map(|k| b.get(*k).and_then(|c| c.get("value")).and_then(Value::as_str).map(String::from))
                    .collect()
            })
            .collect())
    }

    /// The k images that look most like this one, most alike first, by
    /// pand's own vector index (`POST /search` with `like`). The image itself
    /// is not among them. Errors keep pand's status, so "no embedding yet"
    /// (404) can be told apart from pand being down.
    pub async fn like(&self, store: &str, id: &str, k: usize) -> Result<Vec<(String, f32)>, (u16, String)> {
        let r = self
            .http
            .post(format!("{}/search", self.base))
            .json(&json!({ "store": store, "like": format!("<pan/Image/{id}>"), "k": k }))
            .send()
            .await
            .map_err(|e| (502, self.down(e)))?;
        let status = r.status();
        if !status.is_success() {
            return Err((status.as_u16(), r.text().await.unwrap_or_default()));
        }
        let v: Value = r.json().await.map_err(|e| (502, format!("pand /search: {e}")))?;
        Ok(v["hits"]
            .as_array()
            .map(|hits| {
                hits.iter()
                    .filter_map(|h| {
                        let id = h["id"].as_str()?.trim_start_matches("<pan/Image/").trim_end_matches('>');
                        Some((id.to_string(), h["score"].as_f64()? as f32))
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn set(&self, id: &str, fields: Value) -> Result<(), String> {
        self.post(&format!("/media/{id}/set"), fields).await
    }

    pub async fn unset(&self, id: &str, names: &[&str]) -> Result<(), String> {
        self.post(&format!("/media/{id}/unset"), json!(names)).await
    }

    async fn post(&self, path: &str, body: Value) -> Result<(), String> {
        let r = self
            .http
            .post(format!("{}{path}", self.base))
            .json(&body)
            .send()
            .await
            .map_err(|e| self.down(e))?;
        let status = r.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(format!("pand {path}: {status}: {}", r.text().await.unwrap_or_default()))
        }
    }

    fn down(&self, e: reqwest::Error) -> String {
        if e.is_connect() {
            format!("pand is not answering on {}. Is it running?", self.base)
        } else {
            format!("pand: {e}")
        }
    }
}

/// Store and image ids go into URLs and queries unescaped, so only plain
/// ones are accepted. pand's own ids are lowercase letters and digits.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 200 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
        && !id.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert!(valid_id("700c5b"));
        assert!(valid_id("bgsqxfv6"));
        assert!(valid_id("20260906-thesewingloft-take72-square"));
        assert!(!valid_id(""));
        assert!(!valid_id("../x"));
        assert!(!valid_id("a/b"));
        assert!(!valid_id("a b"));
        assert!(!valid_id(".hidden"));
    }
}
