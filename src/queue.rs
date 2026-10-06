//! The save queue: a key press is answered at once, and Pan catches up.
//!
//! Saving a rating in Pan rewrites the image's own metadata as well as the
//! graph, which is right (the rating travels with the file) and too slow to
//! wait for between key presses. So a mark lands here first: the index is
//! updated, the queue is written to disk, and a background task hands each
//! mark to pand. If pand is down the marks wait, on disk, and go when it is
//! back.
//!
//! This is not a second database. The queue holds only marks Pan has not
//! taken yet, and empties as it drains. Pan stays the one record.

use crate::index::{PICK, REJECT};
use crate::pand::Pand;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

/// What a person asked for on one image. A field left as None was not
/// touched, so pressing 3 never clears a flag and pressing P never clears
/// stars.
#[derive(Serialize, Deserialize, Clone, Copy, Default, PartialEq, Debug)]
pub struct Mark {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag: Option<i8>,
}

impl Mark {
    /// A later mark on the same image: its fields win, untouched ones stay.
    pub fn then(self, later: Mark) -> Mark {
        Mark { rating: later.rating.or(self.rating), flag: later.flag.or(self.flag) }
    }

    /// The two calls that say this to pand: what to set, and what to unset.
    /// Zero stars is "no rating" and a neutral flag is "neither", so both
    /// are removals: the ontology says never to write false for unknown.
    pub fn to_pand(self) -> (Map<String, Value>, Vec<&'static str>) {
        let mut set = Map::new();
        let mut unset = Vec::new();
        match self.rating {
            Some(0) => unset.push("rating"),
            Some(n) => {
                set.insert("rating".into(), json!(n.min(5)));
            }
            None => {}
        }
        match self.flag {
            Some(PICK) => {
                set.insert("isPicked".into(), json!(true));
                unset.push("isRejected");
            }
            Some(REJECT) => {
                set.insert("isRejected".into(), json!(true));
                unset.push("isPicked");
            }
            Some(_) => unset.extend(["isPicked", "isRejected"]),
            None => {}
        }
        (set, unset)
    }
}

#[derive(Serialize, Deserialize, Default)]
struct OnDisk {
    /// "<store>/<image>" → the mark still to be saved.
    pending: BTreeMap<String, Mark>,
}

pub struct Queue {
    path: PathBuf,
    inner: Mutex<State>,
    wake: Notify,
}

#[derive(Default)]
struct State {
    pending: BTreeMap<String, Mark>,
    saved: u64,
    last_error: Option<String>,
}

#[derive(Serialize)]
pub struct Status {
    pub pending: usize,
    pub saved: u64,
    pub last_error: Option<String>,
}

impl Queue {
    /// Open the queue file, keeping whatever an earlier run had not saved.
    pub fn open(path: PathBuf) -> Arc<Queue> {
        let pending = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<OnDisk>(&b).ok())
            .map(|d| d.pending)
            .unwrap_or_default();
        Arc::new(Queue { path, inner: Mutex::new(State { pending, ..Default::default() }), wake: Notify::new() })
    }

    pub fn push(&self, store: &str, id: &str, m: Mark) -> Result<(), String> {
        {
            let mut s = self.inner.lock().unwrap();
            let k = format!("{store}/{id}");
            let merged = s.pending.get(&k).copied().unwrap_or_default().then(m);
            s.pending.insert(k, merged);
            self.persist(&s)?;
        }
        self.wake.notify_one();
        Ok(())
    }

    /// Marks for one store that Pan has not taken yet, to lay over a fresh
    /// index: otherwise a rebuild would briefly show the old value.
    pub fn pending_for(&self, store: &str) -> Vec<(String, Mark)> {
        let prefix = format!("{store}/");
        let s = self.inner.lock().unwrap();
        s.pending
            .iter()
            .filter_map(|(k, m)| k.strip_prefix(&prefix).map(|id| (id.to_string(), *m)))
            .collect()
    }

    pub fn status(&self) -> Status {
        let s = self.inner.lock().unwrap();
        Status { pending: s.pending.len(), saved: s.saved, last_error: s.last_error.clone() }
    }

    /// Written whole each time, through a temporary file and a rename, so a
    /// crash leaves the old queue or the new one and never half of either.
    fn persist(&self, s: &State) -> Result<(), String> {
        let body = serde_json::to_vec_pretty(&OnDisk { pending: s.pending.clone() }).map_err(|e| e.to_string())?;
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, body).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path).map_err(|e| format!("{}: {e}", self.path.display()))
    }

    /// Hand marks to pand one at a time, oldest key first, forever.
    pub async fn drain(self: Arc<Queue>, pand: Pand) {
        loop {
            let next = { self.inner.lock().unwrap().pending.iter().next().map(|(k, m)| (k.clone(), *m)) };
            let Some((key, mark)) = next else {
                self.wake.notified().await;
                continue;
            };
            let id = key.split_once('/').map(|(_, id)| id).unwrap_or(&key).to_string();
            let (set, unset) = mark.to_pand();
            let mut r = Ok(());
            if !set.is_empty() {
                r = pand.set(&id, Value::Object(set)).await;
            }
            if r.is_ok() && !unset.is_empty() {
                r = pand.unset(&id, &unset).await;
            }
            let backoff = {
                let mut s = self.inner.lock().unwrap();
                match r {
                    Ok(()) => {
                        // Only if nobody changed it while it was in flight; a
                        // newer mark goes round again.
                        if s.pending.get(&key) == Some(&mark) {
                            s.pending.remove(&key);
                        }
                        s.saved += 1;
                        s.last_error = self.persist(&s).err();
                        false
                    }
                    Err(e) if e.contains("404") => {
                        // pand does not know this image any more. Retrying
                        // would block every mark behind it, so it is dropped,
                        // and said.
                        s.pending.remove(&key);
                        s.last_error = Some(format!("dropped a mark for {id}: {e}"));
                        let _ = self.persist(&s);
                        false
                    }
                    Err(e) => {
                        s.last_error = Some(e);
                        true
                    }
                }
            };
            if backoff {
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_fields_win_untouched_stay() {
        let a = Mark { rating: Some(3), flag: None };
        let b = Mark { rating: None, flag: Some(PICK) };
        assert_eq!(a.then(b), Mark { rating: Some(3), flag: Some(PICK) });
        assert_eq!(a.then(b).then(Mark { rating: Some(0), flag: None }), Mark { rating: Some(0), flag: Some(PICK) });
    }

    #[test]
    fn zero_and_neutral_are_removals() {
        let (set, unset) = Mark { rating: Some(0), flag: Some(0) }.to_pand();
        assert!(set.is_empty());
        assert_eq!(unset, ["rating", "isPicked", "isRejected"]);
        let (set, unset) = Mark { rating: Some(5), flag: Some(REJECT) }.to_pand();
        assert_eq!(set.get("rating"), Some(&json!(5)));
        assert_eq!(set.get("isRejected"), Some(&json!(true)));
        assert_eq!(unset, ["isPicked"]);
    }
}
