#!/usr/bin/env python3
"""lab_build — the precomputed data behind pan-ui's Lab views, for one store.

Reads what Pan already wrote beside a store's images and never writes to
the store. Everything lands in ~/.pan-ui/lab/<store>/, and can be deleted and
rebuilt at any time:

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

Usage:
  tools/.venv/bin/python tools/lab_build.py <store-id> [--pan-ui http://127.0.0.1:8890]

The store's root comes from pan-ui's /api/stores; image times from its
/api/stores/<id>/images, so the Lab and the Library agree about when.
"""
from __future__ import annotations

import argparse
import json
import sys
import time
import urllib.request
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


def image_id(stem: str) -> str:
    # 20260924-205524-7bi2q5rf.embed.qwen3-vl-embedding-2b -> 7bi2q5rf
    return stem.split(".")[0].split("-")[-1]


def load_vectors(root: Path):
    files = sorted((root / "_ignore/media/image/enrichment/embed").rglob("*.npy"))
    ids, rows = [], []
    seen = set()
    for i, f in enumerate(files):
        iid = image_id(f.name)
        if iid in seen:  # a second model's vector for the same image
            continue
        try:
            v = np.load(f)
        except Exception:
            continue
        if v.ndim != 1:
            continue
        if rows and v.shape[0] != rows[0].shape[0]:
            continue
        seen.add(iid)
        ids.append(iid)
        rows.append(v.astype(np.float32))
        if i % 10000 == 0:
            log(f"  read {i:,} of {len(files):,} vectors")
    return ids, np.vstack(rows)


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
def poses(root: Path, index: dict[str, int]):
    files = sorted((root / "_ignore/media/image/enrichment/pose").rglob("*.nq"))
    rows, which, people = [], [], []
    for i, f in enumerate(files):
        iid = image_id(f.name)
        best, best_area, count = None, -1.0, 0
        for line in f.read_text(errors="replace").splitlines():
            if "/ontology/pan/keypoints>" not in line:
                continue
            q = line.split('"')
            if len(q) < 2:
                continue
            try:
                pts = np.array([[float(c) for c in p.split(",")] for p in q[1].split(";") if p], np.float32)
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
        if i % 20000 == 0:
            log(f"  read {i:,} of {len(files):,} pose files")
    return which, people, (np.stack(rows) if rows else np.zeros((0, 17, 3), np.float32))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("store")
    ap.add_argument("--pan-ui", default="http://127.0.0.1:8890")
    ap.add_argument("--out", default=str(Path.home() / ".pan-ui/lab"))
    a = ap.parse_args()

    stores = {s["id"]: s for s in get(f"{a.pan_ui}/api/stores")}
    if a.store not in stores:
        sys.exit(f"no store {a.store}; pan-ui knows {', '.join(stores)}")
    root = Path(stores[a.store]["root"])
    out = Path(a.out) / a.store
    out.mkdir(parents=True, exist_ok=True)
    log(f"store {a.store} ({stores[a.store]['name']}) at {root}")

    cols = get(f"{a.pan_ui}/api/stores/{a.store}/images")
    when = dict(zip(cols["ids"], cols["t"]))

    log("reading vectors")
    ids, x = load_vectors(root)
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
    pid, ppl, pz = poses(root, {i: n for n, i in enumerate(ids)})
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
