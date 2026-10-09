#!/usr/bin/env python3
"""Freeze upstream LongMemEval-S bytes and a deterministic 50-row diagnostic sample."""
import hashlib
import json
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
DEST = ROOT / ".cache/longmemeval-input"
DATA_REV = "98d7416c24c778c2fee6e6f3006e7a073259d48f"
CODE_REV = "9e0b455f4ef0e2ab8f2e582289761153549043fc"
DEST.mkdir(parents=True, exist_ok=True)

def fetch(name, url, ceiling):
    path = DEST / name
    if not path.exists():
        with urllib.request.urlopen(url, timeout=180) as response:
            data = response.read(ceiling + 1)
        assert len(data) <= ceiling, "Unexpected upstream size"
        path.write_bytes(data)
    return path.read_bytes()

data = fetch("longmemeval_s_cleaned.json", f"https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned/resolve/{DATA_REV}/longmemeval_s_cleaned.json", 400_000_000)
rows = json.loads(data)
assert len(rows) == 500
types = sorted({r["question_type"] for r in rows})
assert len(types) == 6
# Seven ordinary rows of each type and eight abstention rows: fixed before calls.
groups = [[r for r in rows if r["question_type"] == t and not r["question_id"].endswith("_abs")][:7] for t in types]
abstention = [r for r in rows if r["question_id"].endswith("_abs")][:8]
sample = [g[0] for g in groups] + abstention[:1] + [r for g in groups for r in g[1:]] + abstention[1:]
assert len(sample) == 50 and len({r["question_id"] for r in sample}) == 50
hashes = {"longmemeval_s_cleaned.json": hashlib.sha256(data).hexdigest()}
for name in ["evaluate_qa.py", "print_qa_metrics.py"]:
    raw = fetch(name, f"https://raw.githubusercontent.com/xiaowu0162/LongMemEval/{CODE_REV}/src/evaluation/{name}", 100_000)
    hashes[name] = hashlib.sha256(raw).hexdigest()
subset = (json.dumps(sample, ensure_ascii=False, separators=(",", ":")) + "\n").encode()
hashes["subset-50.json"] = hashlib.sha256(subset).hexdigest()
manifest = {"dataset": "xiaowu0162/longmemeval-cleaned", "data_revision": DATA_REV, "upstream_commit": CODE_REV, "sha256": hashes, "selection": "First seven non-abstention rows per sorted type, first eight abstention rows; pilot first seven covers all types and abstention", "question_ids": [r["question_id"] for r in sample], "pilot": 7, "questions": 50}
path = DEST / "manifest.json"
if path.exists():
    assert json.loads(path.read_text()) == manifest, "Frozen input drift; inspect before dispatch"
else:
    path.write_text(json.dumps(manifest, indent=2) + "\n")
(DEST / "subset-50.json").write_bytes(subset)
print(json.dumps({"manifest": str(path.relative_to(ROOT)), "bytes": len(data), "sample_sessions": sum(len(r["haystack_sessions"]) for r in sample), "sha256": hashes}))
