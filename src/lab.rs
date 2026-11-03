//! The Lab's data: what `tools/lab_build.py` precomputed for a store, held in
//! memory so the Lab can ask "what looks like this?" and get an answer in
//! milliseconds rather than the second a round trip through pand takes.
//!
//! Everything here is read-only and rebuildable. The files live in
//! `<data>/lab/<store>/`; when `meta.json` changes on disk the store is read
//! again on the next request.
//!
//! The vectors are the store's own embeddings, reduced by PCA to a few
//! hundred dimensions and normalised. Ranking in that space is close to, not
//! the same as, ranking in the model's full space; the Lab says so where it
//! shows a score.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// The files a page may fetch, and nothing else from that folder.
pub const FILES: &[&str] = &["meta.json", "atlas.f32", "nbr.u32", "nbr.f32", "echoes.json", "pose.f32", "pose.json"];

pub struct Lab {
    pub ids: Vec<String>,
    pub pos: HashMap<String, u32>,
    pub dim: usize,
    pub vec: Vec<f32>,
    stamp: Option<SystemTime>,
}

#[derive(Deserialize)]
struct Meta {
    ids: Vec<String>,
    dim: usize,
}

#[derive(Default)]
pub struct Labs {
    cache: Mutex<HashMap<String, Arc<Lab>>>,
}

pub fn dir(data: &Path, store: &str) -> PathBuf {
    data.join("lab").join(store)
}

fn stamp(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

impl Labs {
    /// The store's Lab data, read from disk the first time and whenever it
    /// has been rebuilt since.
    pub fn get(&self, data: &Path, store: &str) -> Result<Arc<Lab>, String> {
        let d = dir(data, store);
        let now = stamp(&d.join("meta.json"));
        if now.is_none() {
            return Err(format!(
                "no Lab data for store {store} yet: run tools/.venv/bin/python tools/lab_build.py {store}"
            ));
        }
        if let Some(l) = self.cache.lock().unwrap().get(store) {
            if l.stamp == now {
                return Ok(Arc::clone(l));
            }
        }
        let meta: Meta = serde_json::from_slice(&std::fs::read(d.join("meta.json")).map_err(|e| e.to_string())?)
            .map_err(|e| format!("meta.json: {e}"))?;
        let raw = std::fs::read(d.join("vec.f32")).map_err(|e| format!("vec.f32: {e}"))?;
        let vec: Vec<f32> = raw.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
        if vec.len() != meta.ids.len() * meta.dim {
            return Err(format!(
                "vec.f32 holds {} numbers, meta.json promises {} × {}",
                vec.len(),
                meta.ids.len(),
                meta.dim
            ));
        }
        let pos = meta.ids.iter().enumerate().map(|(i, id)| (id.clone(), i as u32)).collect();
        let lab = Arc::new(Lab { ids: meta.ids, pos, dim: meta.dim, vec, stamp: now });
        self.cache.lock().unwrap().insert(store.to_string(), Arc::clone(&lab));
        Ok(lab)
    }
}

impl Lab {
    pub fn row(&self, i: usize) -> &[f32] {
        &self.vec[i * self.dim..(i + 1) * self.dim]
    }

    pub fn of(&self, id: &str) -> Option<&[f32]> {
        self.pos.get(id).map(|&i| self.row(i as usize))
    }

    /// The k rows most similar to q, best first, skipping `skip`.
    pub fn nearest(&self, q: &[f32], k: usize, skip: &HashSet<usize>) -> Vec<(usize, f32)> {
        let mut best: Vec<(usize, f32)> = Vec::with_capacity(k + 1);
        let mut floor = f32::NEG_INFINITY;
        for i in 0..self.ids.len() {
            let s = dot(self.row(i), q);
            if best.len() >= k && s <= floor {
                continue;
            }
            if skip.contains(&i) {
                continue;
            }
            let at = best.partition_point(|&(_, b)| b >= s);
            best.insert(at, (i, s));
            if best.len() > k {
                best.pop();
            }
            if best.len() >= k {
                floor = best[k - 1].1;
            }
        }
        best
    }

    /// The two directions in which the images around `centre` differ most:
    /// the top principal components of its m nearest neighbours.
    pub fn local_axes(&self, centre: &[f32], m: usize) -> [Vec<f32>; 2] {
        let near = self.nearest(centre, m, &HashSet::new());
        let n = near.len().max(1) as f32;
        let mut mean = vec![0f32; self.dim];
        for &(i, _) in &near {
            for (a, b) in mean.iter_mut().zip(self.row(i)) {
                *a += b / n;
            }
        }
        let rows: Vec<Vec<f32>> =
            near.iter().map(|&(i, _)| self.row(i).iter().zip(&mean).map(|(a, b)| a - b).collect()).collect();
        let first = power(&rows, self.dim, None);
        let second = power(&rows, self.dim, Some(&first));
        [first, second]
    }
}

pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

pub fn normalise(v: &mut [f32]) {
    let n = dot(v, v).sqrt();
    if n > 0.0 {
        v.iter_mut().for_each(|x| *x /= n);
    }
}

/// The top principal direction of the rows, by power iteration; with
/// `away`, the top one at right angles to it.
fn power(rows: &[Vec<f32>], dim: usize, away: Option<&[f32]>) -> Vec<f32> {
    // A fixed start, not a random one: the same image gives the same axes.
    let mut v: Vec<f32> = (0..dim).map(|i| ((i * 7919 % 97) as f32 / 97.0) - 0.5).collect();
    for _ in 0..40 {
        if let Some(a) = away {
            let p = dot(&v, a);
            v.iter_mut().zip(a).for_each(|(x, y)| *x -= p * y);
        }
        normalise(&mut v);
        let mut next = vec![0f32; dim];
        for r in rows {
            let p = dot(r, &v);
            next.iter_mut().zip(r).for_each(|(x, y)| *x += p * y);
        }
        v = next;
    }
    if let Some(a) = away {
        let p = dot(&v, a);
        v.iter_mut().zip(a).for_each(|(x, y)| *x -= p * y);
    }
    normalise(&mut v);
    v
}

/// A small random number generator for jitter, so the Lab needs no crate
/// for it. Seeded from the clock.
pub struct Rng(u64);

impl Rng {
    pub fn new() -> Self {
        let t = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
        Rng(t | 1)
    }
    pub fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
    /// Roughly normal, mean 0, spread 1.
    pub fn gauss(&mut self) -> f32 {
        (0..6).map(|_| self.next()).sum::<f32>() - 3.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lab(rows: &[[f32; 3]]) -> Lab {
        let ids: Vec<String> = (0..rows.len()).map(|i| format!("i{i}")).collect();
        let pos = ids.iter().enumerate().map(|(i, s)| (s.clone(), i as u32)).collect();
        let mut vec = Vec::new();
        for r in rows {
            let mut r = r.to_vec();
            normalise(&mut r);
            vec.extend(r);
        }
        Lab { ids, pos, dim: 3, vec, stamp: None }
    }

    #[test]
    fn nearest_is_best_first_and_skips() {
        let l = lab(&[[1.0, 0.0, 0.0], [0.9, 0.1, 0.0], [0.0, 1.0, 0.0], [0.7, 0.7, 0.0]]);
        let q = l.row(0).to_vec();
        let got: Vec<usize> = l.nearest(&q, 3, &HashSet::new()).iter().map(|x| x.0).collect();
        assert_eq!(got, vec![0, 1, 3]);
        let skip: HashSet<usize> = [0].into();
        let got: Vec<usize> = l.nearest(&q, 2, &skip).iter().map(|x| x.0).collect();
        assert_eq!(got, vec![1, 3]);
    }

    #[test]
    fn first_axis_follows_the_spread() {
        // Points spread along x, barely along y.
        let rows: Vec<Vec<f32>> = vec![vec![2.0, 0.1, 0.0], vec![-2.0, -0.1, 0.0], vec![1.0, 0.0, 0.0], vec![-1.0, 0.05, 0.0]];
        let a = power(&rows, 3, None);
        assert!(a[0].abs() > 0.99, "{a:?}");
        let b = power(&rows, 3, Some(&a));
        assert!(dot(&a, &b).abs() < 1e-4);
    }
}
