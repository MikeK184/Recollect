#!/usr/bin/env python3
"""Fetch the frozen public benchmark input; never silently accept corpus drift."""
import hashlib
from pathlib import Path
import urllib.request

URL = "https://datasets-server.huggingface.co/rows?dataset=hotpotqa/hotpot_qa&config=distractor&split=validation&offset=0&length=50"
SHA256 = "32dd92947d6c50194bc2e76588bc78ad6ad08805a4b05728336ac70cd7e19b96"
ROOT = Path(__file__).resolve().parent.parent
DEST = ROOT / ".cache/public-benchmark-input/hotpotqa-validation-50.json"

if DEST.exists():
    data = DEST.read_bytes()
else:
    with urllib.request.urlopen(URL, timeout=40) as response:
        data = response.read(2_000_001)
    assert len(data) <= 2_000_000, "Unexpected dataset size"
assert hashlib.sha256(data).hexdigest() == SHA256, "Dataset drift: preserve and review the frozen corpus before a new experiment"
DEST.parent.mkdir(parents=True, exist_ok=True)
if not DEST.exists():
    DEST.write_bytes(data)
print("Verified 50 frozen HotpotQA validation rows (CC BY-SA 4.0):", DEST.relative_to(ROOT))
