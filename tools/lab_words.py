#!/usr/bin/env python3
"""lab_words — every image's caption, for the Lab's word search.

Asks pand once for every fact whose property name ends in "caption", in any
vocabulary, and keeps the longest per image. Asking takes about a minute
on a store of 200,000 images, which is why it is done here and not live.

Writes ~/.pan-ui/lab/<store>/words.json: {"ids": [...], "text": [...]}.

Usage: tools/.venv/bin/python tools/lab_words.py <store-id> [--pand http://127.0.0.1:7401]
"""
from __future__ import annotations

import argparse
import json
import time
import urllib.request
from pathlib import Path

QUERY = """SELECT ?m ?c WHERE {
  ?m ?p ?c .
  FILTER(isLiteral(?c) && STRENDS(LCASE(STR(?p)), "caption"))
}"""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("store")
    ap.add_argument("--pand", default="http://127.0.0.1:7401")
    ap.add_argument("--out", default=str(Path.home() / ".pan-ui/lab"))
    a = ap.parse_args()
    t = time.time()
    req = urllib.request.Request(
        f"{a.pand}/stores/{a.store}/sparql",
        data=QUERY.encode(),
        headers={"Content-Type": "application/sparql-query", "Accept": "application/sparql-results+json"},
    )
    with urllib.request.urlopen(req, timeout=900) as r:
        rows = json.load(r)["results"]["bindings"]
    best: dict[str, str] = {}
    for b in rows:
        m = b["m"]["value"].rstrip(">").split("/")[-1]
        c = b["c"]["value"].strip()
        if len(c) > len(best.get(m, "")):
            best[m] = c
    out = Path(a.out) / a.store
    out.mkdir(parents=True, exist_ok=True)
    ids = sorted(best)
    (out / "words.json").write_text(json.dumps({"ids": ids, "text": [best[i] for i in ids]}))
    print(f"{len(rows):,} caption facts, {len(ids):,} images, {time.time() - t:.0f}s")


if __name__ == "__main__":
    main()
