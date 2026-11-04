#!/usr/bin/env python3
"""lab_build — the precomputed data behind pan-ui's Lab views, for one store.

Asks pand for what Pan already knows about a store's images, over HTTP
only, so Pan can be on another machine; it never writes to the store.
Everything lands in ~/.pan-ui/lab/<store>/, and can be deleted and rebuilt
at any time:

  meta.json    ids (row order), times, how it was built
  vec.f32      every image's embedding, reduced to DIM dimensions by PCA and
               normalised, so pan-ui can rank 65,000 images in milliseconds
  atlas.f32    a 2D position per image (UMAP): similar-looking images sit
               close together
  nbr.u32      the K nearest images to each one, by look, and nbr.f32 their
               similarities
  echoes.json  pairs that look alike but were made far apart in time
  pose.f32     one body pose per image that has one (the largest person),
               17 joints of x, y, confidence, normalised to the body; pose.json
               says which image each row is
  raw.f32      the full embeddings as pand sent them, and raw.json their ids
               and when they were asked for: the next build asks pand only
               for what changed since (GET /stores/<id>/embeddings?since=)

Usage:
  tools/.venv/bin/python tools/lab_build.py <store-id> [--pan-ui http://127.0.0.1:8890] [--pand http://127.0.0.1:7401] [--fresh]

Embeddings come from pand's GET /stores/<id>/embeddings, poses from a SPARQL
query on pand. Image times come from pan-ui's /api/stores/<id>/images, so the
Lab and the Library agree about when.
"""
from __future__ import annotations

import argparse
import json
import sys
import time
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

import numpy as np

DIM = 256
K = 16
ECHO_GAP_DAYS = 21


def log(*a):
    print(time.strftime("%H:%M:%S"), *a, flush=True)


def get(url):
    with urllib.request.urlopen(url, timeout=600) as r:
        return json.load(r)


def image_id(iri: str) -> str:
    # <pan/Image/7bi2q5rf> or https://repolex.ai/pan/Image/7bi2q5rf -> 7bi2q5rf
    return iri.rstrip(">").rsplit("/", 1)[-1]


def fetch_vectors(pand: str, store: str, since: str | None):
    """Every embedding pand has for the store, or only those produced at or
    after `since`: one JSON line per image, read as it streams."""
    url = f"{pand}/stores/{store}/embeddings"
    if since:
        url += "?since=" + urllib.parse.quote(since)
    ids, rows = [], []
    with urllib.request.urlopen(url, timeout=3600) as r:
        total = int(r.headers.get("X-Pan-Count") or 0)
        for n, line in enumerate(r):
            # {"id":"<pan/Image/x>","vector":[...]}: the numbers are parsed
            # straight from the text, much faster than a JSON decode per row.
            head, _, tail = line.partition(b'"vector":[')
            iid = image_id(head.split(b'"id":"', 1)[1].split(b'"', 1)[0].decode())
            ids.append(iid)
            rows.append(np.array(tail.rstrip().rstrip(b"}").rstrip(b"]").split(b","), dtype=np.float32))
            if n and n % 10000 == 0:
                log(f"  {n:,} of {total:,} vectors")
    return ids, rows


def load_vectors(pand: str, store: str, out: Path, fresh: bool):
    """The store's embeddings, kept between builds: the first build fetches
    them all, later ones only what pand produced since the last ask, and
    overwrite by id (a re-embedded image replaces its old vector)."""
    cache, cmeta = out / "raw.f32", out / "raw.json"
    asked = datetime.now(timezone.utc).isoformat(timespec="seconds")
    if not fresh and cache.exists() and cmeta.exists():
        m = json.loads(cmeta.read_text())
        x = np.fromfile(cache, np.float32).reshape(len(m["ids"]), m["dim"])
        ids = list(m["ids"])
        log(f"  {len(ids):,} kept from the last build; asking pand for what changed since {m['asked']}")
        new_ids, new_rows = fetch_vectors(pand, store, m["asked"])
        at = {i: n for n, i in enumerate(ids)}
        add = []
        for i, v in zip(new_ids, new_rows):
            if i in at:
                x[at[i]] = v
            else:
                at[i] = len(ids) + len(add)
                add.append((i, v))
        if add:
            ids += [i for i, _ in add]
            x = np.vstack([x, np.stack([v for _, v in add])])
        log(f"  {len(new_ids):,} changed: {len(add):,} new, {len(new_ids) - len(add):,} re-embedded")
    else:
        ids, rows = fetch_vectors(pand, store, None)
        x = np.stack(rows)
        del rows
    x.tofile(cache)
    cmeta.write_text(json.dumps({"ids": ids, "dim": int(x.shape[1]), "asked": asked}))
    return ids, x


def pca(x: np.ndarray, dim: int):
    mean = x.mean(0)
    sample = x[:: max(1, len(x) // 20000)] - mean
    _, _, vt = np.linalg.svd(sample, full_matrices=False)
    comps = vt[:dim]
    y = (x - mean) @ comps.T
    y /= np.linalg.norm(y, axis=1, keepdims=True) + 1e-9
    return y.astype(np.float32), mean, comps


def neighbours(y: np.ndarray, k: int):
    n = len(y)
    idx = np.zeros((n, k), np.uint32)
    sim = np.zeros((n, k), np.float32)
    step = 1024
    for a in range(0, n, step):
        s = y[a : a + step] @ y.T
        for r in range(s.shape[0]):
            s[r, a + r] = -2  # not yourself
        part = np.argpartition(-s, k, axis=1)[:, :k]
        ps = np.take_along_axis(s, part, 1)
        order = np.argsort(-ps, axis=1)
        idx[a : a + step] = np.take_along_axis(part, order, 1)
        sim[a : a + step] = np.take_along_axis(ps, order, 1)
        if (a // step) % 8 == 0:
            log(f"  neighbours {a:,} of {n:,}")
    return idx, sim


def echoes(y, t, gap_days, per=400):
    """For each image, its most similar image made at least gap_days away.
    Keep the strongest pairs, each image at most once."""
    n = len(y)
    gap = gap_days * 86400
    best = []
    step = 1024
    tt = np.asarray(t, np.float64)
    for a in range(0, n, step):
        s = y[a : a + step] @ y.T
        far = np.abs(tt[a : a + step, None] - tt[None, :]) >= gap
        s = np.where(far, s, -2)
        j = s.argmax(1)
        v = s[np.arange(len(j)), j]
        for r in range(len(j)):
            if v[r] > -1:
                best.append((float(v[r]), a + r, int(j[r])))
    best.sort(reverse=True)
    used, out = set(), []
    for v, i, j in best:
        if i in used or j in used:
            continue
        used.add(i)
        used.add(j)
        out.append({"a": i, "b": j, "sim": round(v, 4)})
        if len(out) >= per * 10:
            break
    return out


# COCO-17 body order, the first 17 of RTMW's 133 whole-body points.
POSE_QUERY = """PREFIX pan: <https://repolex.ai/ontology/pan/>
SELECT ?img ?k WHERE { ?img pan:poseData ?e . ?e pan:item ?pose . ?pose pan:keypoints ?k }"""


def sparql(pand: str, store: str, query: str):
    req = urllib.request.Request(
        f"{pand}/stores/{store}/sparql",
        query.encode(),
        {"Content-Type": "application/sparql-query", "Accept": "application/sparql-results+json"},
    )
    with urllib.request.urlopen(req, timeout=3600) as r:
        return json.load(r)["results"]["bindings"]


def poses(pand: str, store: str):
    """One row per person pand's pose stage found; each image keeps its
    largest person."""
    people_of: dict[str, list] = {}
    for b in sparql(pand, store, POSE_QUERY):
        people_of.setdefault(image_id(b["img"]["value"]), []).append(b["k"]["value"])
    log(f"  {sum(map(len, people_of.values())):,} people in {len(people_of):,} images")
    rows, which, people = [], [], []
    for i, (iid, found) in enumerate(people_of.items()):
        best, best_area, count = None, -1.0, 0
        for k in found:
            try:
                pts = np.array([[float(c) for c in p.split(",")] for p in k.split(";")[:17] if p], np.float32)
            except ValueError:
                continue
            if len(pts) < 17:
                continue
            body = pts[:17]
            count += 1
            good = body[body[:, 2] > 1.0]
            if len(good) < 6:
                continue
            area = float(np.ptp(good[:, 0]) * np.ptp(good[:, 1]))
            if area > best_area:
                best, best_area = body, area
        if best is None:
            continue
        # Normalise: centre on the hips (or the shoulders), scale by torso.
        conf = best[:, 2]
        xy = best[:, :2].copy()
        hip = xy[[11, 12]][conf[[11, 12]] > 1.0]
        sho = xy[[5, 6]][conf[[5, 6]] > 1.0]
        if len(sho) == 0:
            continue
        c = hip.mean(0) if len(hip) else sho.mean(0)
        scale = np.linalg.norm(sho.mean(0) - hip.mean(0)) if len(hip) else np.ptp(xy[conf > 1.0], axis=0).max() / 3
        if not scale or scale < 1:
            continue
        xy = (xy - c) / scale
        rows.append(np.concatenate([xy, conf[:, None]], 1).astype(np.float32))
        which.append(iid)
        people.append(count)
        if i and i % 20000 == 0:
            log(f"  {i:,} of {len(people_of):,} images")
    return which, people, (np.stack(rows) if rows else np.zeros((0, 17, 3), np.float32))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("store")
    ap.add_argument("--pan-ui", default="http://127.0.0.1:8890")
    ap.add_argument("--pand", default="http://127.0.0.1:7401")
    ap.add_argument("--fresh", action="store_true", help="fetch every embedding again instead of only what changed")
    ap.add_argument("--out", default=str(Path.home() / ".pan-ui/lab"))
    a = ap.parse_args()

    stores = {s["id"]: s for s in get(f"{a.pan_ui}/api/stores")}
    if a.store not in stores:
        sys.exit(f"no store {a.store}; pan-ui knows {', '.join(stores)}")
    out = Path(a.out) / a.store
    out.mkdir(parents=True, exist_ok=True)
    log(f"store {a.store} ({stores[a.store]['name']}) from pand at {a.pand}")

    cols = get(f"{a.pan_ui}/api/stores/{a.store}/images")
    when = dict(zip(cols["ids"], cols["t"]))

    log("embeddings from pand")
    ids, x = load_vectors(a.pand, a.store, out, a.fresh)
    t = [when.get(i, 0) for i in ids]
    log(f"{len(ids):,} vectors of {x.shape[1]} dimensions; {sum(1 for v in t if v == 0):,} without a date")

    log(f"PCA to {DIM}")
    y, mean, comps = pca(x, DIM)
    del x
    y.tofile(out / "vec.f32")

    log(f"{K} nearest neighbours each")
    nbr, nsim = neighbours(y, K)
    nbr.tofile(out / "nbr.u32")
    nsim.tofile(out / "nbr.f32")

    log("echoes")
    ech = echoes(y, t, ECHO_GAP_DAYS)
    (out / "echoes.json").write_text(json.dumps({"gap_days": ECHO_GAP_DAYS, "pairs": ech}))
    log(f"  {len(ech):,} pairs")

    log("poses")
    pid, ppl, pz = poses(a.pand, a.store)
    pz.tofile(out / "pose.f32")
    (out / "pose.json").write_text(json.dumps({"ids": pid, "people": ppl}))
    log(f"  {len(pid):,} poses")

    meta = {
        "store": a.store,
        "n": len(ids),
        "dim": DIM,
        "k": K,
        "built": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "ids": ids,
        "t": t,
        "atlas": False,
    }
    (out / "meta.json").write_text(json.dumps(meta))

    log("UMAP to 2D (the slow part)")
    import umap

    pos = umap.UMAP(n_neighbors=30, min_dist=0.08, metric="cosine", low_memory=True, verbose=False).fit_transform(y[:, :64])
    pos = (pos - pos.min(0)) / (np.ptp(pos, 0) + 1e-9)
    pos.astype(np.float32).tofile(out / "atlas.f32")
    meta["atlas"] = True
    (out / "meta.json").write_text(json.dumps(meta))
    log("done")


if __name__ == "__main__":
    main()
