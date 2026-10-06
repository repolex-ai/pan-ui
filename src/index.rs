//! A store's index: every image, its date, its sets, its rating and flag.
//!
//! pand answers a whole-store listing in about two seconds per query on a
//! store of 200,000 images. That is fine once and far too slow per click, so
//! pan-ui asks once, keeps the answer in memory, and keeps a copy on disk so
//! the next start shows something at once. The copy is disposable: delete it
//! and the next start asks pand again. Ratings and flags are never decided
//! here; they live on the image in Pan.

use crate::pand::{IMAGE_PREFIX, PAN, Pand, SET_PREFIX};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Flag values, Lightroom's three: picked (white flag), none, rejected
/// (black flag).
pub const PICK: i8 = 1;
pub const REJECT: i8 = -1;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Index {
    pub store: String,
    /// When pand was asked, RFC3339.
    pub built: String,
    /// Images in time order, oldest first.
    pub ids: Vec<String>,
    /// Unix seconds of `pan:mediaCreatedDate`.
    pub t: Vec<i64>,
    pub rating: Vec<u8>,
    pub flag: Vec<i8>,
    pub sets: Vec<SetInfo>,
    /// Image positions per set, in time order. Same order as `sets`.
    pub members: Vec<Vec<u32>>,
    /// Images with no readable date, left out of the listing.
    pub undated: usize,
    #[serde(skip)]
    pub pos: HashMap<String, u32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SetInfo {
    pub id: String,
    pub count: usize,
    pub first: i64,
    pub last: i64,
}

impl Index {
    /// Ask pand for the three things the listing needs, all at once.
    pub async fn build(pand: &Pand, store: &str) -> Result<Index, String> {
        let dates = format!("PREFIX pan: <{PAN}> SELECT ?m ?d WHERE {{ ?m pan:mediaCreatedDate ?d }}");
        let sets = format!(
            "PREFIX pan: <{PAN}> SELECT ?m ?s WHERE {{ ?m pan:relatedToId ?s . \
             FILTER(STRSTARTS(STR(?s), \"{SET_PREFIX}\")) }}"
        );
        let marks = format!(
            "PREFIX pan: <{PAN}> SELECT ?m ?r ?p ?x WHERE {{ \
             {{ ?m pan:rating ?r }} UNION {{ ?m pan:isPicked ?p }} UNION {{ ?m pan:isRejected ?x }} }}"
        );
        let (dates, sets, marks) = tokio::join!(
            pand.select(store, &["m", "d"], &dates),
            pand.select(store, &["m", "s"], &sets),
            pand.select(store, &["m", "r", "p", "x"], &marks),
        );
        let built = chrono::Utc::now().to_rfc3339();
        Ok(Index::assemble(store, built, dates?, sets?, marks?))
    }

    pub fn assemble(
        store: &str,
        built: String,
        dates: Vec<crate::pand::Row>,
        sets: Vec<crate::pand::Row>,
        marks: Vec<crate::pand::Row>,
    ) -> Index {
        let local = |iri: &str, prefix: &str| iri.strip_prefix(prefix).map(String::from);
        let mut rows: Vec<(i64, String)> = Vec::with_capacity(dates.len());
        let mut undated = 0;
        for r in dates {
            let (Some(m), Some(d)) = (&r[0], &r[1]) else { continue };
            let Some(id) = local(m, IMAGE_PREFIX) else { continue };
            match chrono::DateTime::parse_from_rfc3339(d) {
                Ok(t) => rows.push((t.timestamp(), id)),
                Err(_) => undated += 1,
            }
        }
        // Time order; the id breaks ties so two builds of the same store
        // always list it the same way.
        rows.sort();
        rows.dedup_by(|a, b| a.1 == b.1);
        let n = rows.len();
        let mut ix = Index {
            store: store.to_string(),
            built,
            ids: Vec::with_capacity(n),
            t: Vec::with_capacity(n),
            rating: vec![0; n],
            flag: vec![0; n],
            undated,
            ..Default::default()
        };
        for (t, id) in rows {
            ix.t.push(t);
            ix.ids.push(id);
        }
        ix.reindex();

        for r in marks {
            let Some(id) = r[0].as_deref().and_then(|m| local(m, IMAGE_PREFIX)) else { continue };
            let Some(&p) = ix.pos.get(&id) else { continue };
            let p = p as usize;
            if let Some(v) = r[1].as_deref().and_then(|v| v.parse::<u8>().ok()) {
                ix.rating[p] = v.min(5);
            }
            if r[2].as_deref() == Some("true") {
                ix.flag[p] = PICK;
            }
            // A reject wins over a pick if an image somehow carries both: the
            // safer reading of a contradiction is "someone did not want it".
            if r[3].as_deref() == Some("true") {
                ix.flag[p] = REJECT;
            }
        }

        let mut by_set: HashMap<String, Vec<u32>> = HashMap::new();
        for r in sets {
            let (Some(m), Some(s)) = (&r[0], &r[1]) else { continue };
            let (Some(id), Some(set)) = (local(m, IMAGE_PREFIX), local(s, SET_PREFIX)) else { continue };
            if let Some(&p) = ix.pos.get(&id) {
                by_set.entry(set).or_default().push(p);
            }
        }
        let mut sets: Vec<(SetInfo, Vec<u32>)> = by_set
            .into_iter()
            .map(|(id, mut m)| {
                m.sort_unstable();
                m.dedup();
                let info = SetInfo {
                    id,
                    count: m.len(),
                    first: ix.t[m[0] as usize],
                    last: ix.t[*m.last().unwrap() as usize],
                };
                (info, m)
            })
            .collect();
        // Newest set first: the one you just made is the one you want.
        sets.sort_by(|a, b| b.0.first.cmp(&a.0.first).then_with(|| a.0.id.cmp(&b.0.id)));
        for (info, m) in sets {
            ix.sets.push(info);
            ix.members.push(m);
        }
        ix
    }

    pub fn reindex(&mut self) {
        self.pos = self.ids.iter().enumerate().map(|(i, id)| (id.clone(), i as u32)).collect();
    }

    pub fn set_position(&self, set: &str) -> Option<usize> {
        self.sets.iter().position(|s| s.id == set)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(v: &[Option<&str>]) -> crate::pand::Row {
        v.iter().map(|x| x.map(String::from)).collect()
    }
    fn img(id: &str) -> String {
        format!("{IMAGE_PREFIX}{id}")
    }

    #[test]
    fn assembles_time_order_sets_and_marks() {
        let (a, b, c) = (img("a"), img("b"), img("c"));
        let dates = vec![
            row(&[Some(&c), Some("2026-09-03T10:00:00-07:00")]),
            row(&[Some(&a), Some("2026-09-01T10:00:00-07:00")]),
            row(&[Some(&b), Some("2026-09-02T10:00:00-07:00")]),
            row(&[Some(&img("d")), Some("not a date")]),
        ];
        let s1 = format!("{SET_PREFIX}one");
        let s2 = format!("{SET_PREFIX}two");
        let sets = vec![
            row(&[Some(&c), Some(&s1)]),
            row(&[Some(&a), Some(&s1)]),
            row(&[Some(&b), Some(&s2)]),
            row(&[Some(&img("ghost")), Some(&s2)]),
        ];
        let marks = vec![
            row(&[Some(&a), Some("4"), None, None]),
            row(&[Some(&b), None, Some("true"), None]),
            row(&[Some(&c), None, Some("true"), None]),
            row(&[Some(&c), None, None, Some("true")]),
        ];
        let ix = Index::assemble("s", "now".into(), dates, sets, marks);
        assert_eq!(ix.ids, ["a", "b", "c"]);
        assert_eq!(ix.undated, 1);
        assert_eq!(ix.rating, [4, 0, 0]);
        assert_eq!(ix.flag, [0, PICK, REJECT]);
        // Newest set first; members in time order; the ghost is not a member.
        assert_eq!(ix.sets.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["two", "one"]);
        assert_eq!(ix.members[1], [0, 2]);
        assert_eq!(ix.members[0], [1]);
        assert_eq!(ix.sets[0].count, 1);
    }
}
