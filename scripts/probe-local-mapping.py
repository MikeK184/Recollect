#!/usr/bin/env python3
"""Measure local GLiNER mentions against frozen synthetic annotations.

This is an offline extraction prototype, not a product write path. Model files
may be downloaded first; document inference uses the local model and no API.
"""
from __future__ import annotations

import argparse
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import time

ROOT = Path(__file__).resolve().parents[1]
MODEL = "fastino/gliner2.5-base-v1"
REVISION = "54785d51df8d86a0d8f2eb212fec15871aad2c4d"
FIXTURE = ROOT / "docs/research/automatic-knowledge-mapping-fixtures-2026-10-09.json"


def signature(mention: dict) -> tuple:
    return mention["type"], mention["start"], mention["end"], mention["text"]


def byte_span(text: str, start: int, end: int) -> tuple[int, int]:
    if type(start) is not int or type(end) is not int or not 0 <= start < end <= len(text):
        raise ValueError("Local extractor returned an invalid character span")
    return len(text[:start].encode("utf-8")), len(text[:end].encode("utf-8"))


def corpus() -> dict:
    data = json.loads(FIXTURE.read_text())
    assert len(data["documents"]) == 40
    assert len({d["id"] for d in data["documents"]}) == 40
    for document in data["documents"]:
        assert document["partition"] in {"development", "held_out"}
        assert len(document["text"].encode()) <= 4096
        for mention in document["mentions"]:
            assert mention["type"] in data["entity_schema"]
            assert document["text"][mention["start"]:mention["end"]] == mention["text"]
        assert len({signature(m) for m in document["mentions"]}) == len(document["mentions"])
    return data


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--partition", choices=["development", "held_out"], default="development")
    parser.add_argument("--device", choices=["cpu", "mps"], default="cpu")
    parser.add_argument("--threshold", type=float, default=0.5)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--check-only", action="store_true")
    args = parser.parse_args()
    if not 0 < args.threshold < 1:
        parser.error("threshold must be between zero and one")
    output = args.output.resolve()
    if not output.is_relative_to(ROOT):
        parser.error("output must stay inside the Recollect repository")
    data = corpus()
    if args.check_only:
        # Canonical SourceSpan uses UTF-8 byte coordinates. Keep this separate
        # from the frozen ASCII NER corpus and exercise a non-ASCII prefix.
        sample = "🙂 café uses Vault."
        start = sample.index("Vault")
        first, last = byte_span(sample, start, start + len("Vault"))
        assert first != start
        assert sample.encode("utf-8")[first:last].decode("utf-8") == "Vault"
        for invalid in [(True, 4), (-1, 4), (0, len(sample) + 1), (4, 4)]:
            try:
                byte_span(sample, *invalid)
            except ValueError:
                continue
            raise AssertionError("Invalid offset accepted")
        print(json.dumps({"documents": 40, "annotations_valid": True,
                          "unicode_coordinate_control": True, "provider_calls": 0}))
        return

    runtime = ROOT / ".cache/automatic-mapping"
    os.environ["HF_HOME"] = str(runtime / "huggingface")
    os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"
    os.environ["TOKENIZERS_PARALLELISM"] = "false"
    from huggingface_hub import snapshot_download
    import torch
    from gliner2 import AutoExtractor

    torch.set_num_threads(2)
    started = time.monotonic()
    checkpoint = snapshot_download(
        repo_id=MODEL,
        revision=REVISION,
        local_dir=runtime / "models" / "gliner2.5-base-v1" / REVISION,
        allow_patterns=["*.json", "*.safetensors", "*.model", "*.txt"],
    )
    model = AutoExtractor.from_pretrained(checkpoint, map_location=args.device)
    model.eval()
    load_seconds = time.monotonic() - started
    # All model/tokenizer files are loaded. These flags request offline mode;
    # they are not an operating-system network sandbox or a product adapter.
    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TRANSFORMERS_OFFLINE"] = "1"
    results = []
    for document in data["documents"]:
        if document["partition"] != args.partition:
            continue
        started = time.monotonic()
        with torch.inference_mode():
            extracted = model.extract_entities(
                document["text"],
                data["entity_schema"],
                threshold=args.threshold,
                include_confidence=True,
                include_spans=True,
            )
        predictions = []
        for kind, mentions in extracted.get("entities", {}).items():
            for mention in mentions:
                predicted = {"type": kind, **mention}
                if type(predicted.get("start")) is not int or type(predicted.get("end")) is not int:
                    raise ValueError("Local extractor returned a mention without exact spans")
                if not 0 <= predicted["start"] < predicted["end"] <= len(document["text"]):
                    raise ValueError("Local extractor returned an out-of-range source span")
                if document["text"][predicted["start"]:predicted["end"]] != predicted["text"]:
                    raise ValueError("Local extractor returned an invalid source span")
                first, last = byte_span(document["text"], predicted["start"], predicted["end"])
                predicted["byte_start"], predicted["byte_end"] = first, last
                if document["text"].encode("utf-8")[first:last].decode("utf-8") != predicted["text"]:
                    raise ValueError("Local extractor returned an invalid UTF-8 source span")
                predictions.append(predicted)
        expected = {signature(m) for m in document["mentions"]}
        actual = {signature(m) for m in predictions}
        result = {
            "id": document["id"],
            "seconds": time.monotonic() - started,
            "true_positive": len(expected & actual),
            "false_positive": len(actual - expected),
            "false_negative": len(expected - actual),
            "predictions": predictions,
            "missed": [m for m in document["mentions"] if signature(m) not in actual],
        }
        results.append(result)
        print(json.dumps({k: result[k] for k in ["id", "seconds", "true_positive", "false_positive", "false_negative"]}), flush=True)

    tp = sum(r["true_positive"] for r in results)
    fp = sum(r["false_positive"] for r in results)
    fn = sum(r["false_negative"] for r in results)
    timings = sorted(r["seconds"] for r in results)
    report = {
        "corpus": data["name"], "partition": args.partition,
        "model": MODEL, "revision": REVISION,
        "library": importlib.metadata.version("gliner2"),
        "torch": torch.__version__, "device": args.device,
        "machine": platform.machine(), "python": platform.python_version(),
        "threshold": args.threshold, "schema": data["entity_schema"],
        "span_coordinates": "start/end=unicode-codepoint-v1; byte_start/byte_end=utf8-byte-v1; half-open",
        "paid_inference_calls": 0, "api_cost_usd": 0,
        "load_seconds": load_seconds, "cases": len(results),
        "true_positive": tp, "false_positive": fp, "false_negative": fn,
        "precision": tp / (tp + fp) if tp + fp else 0,
        "recall": tp / (tp + fn) if tp + fn else 0,
        "p95_seconds": timings[max(0, (95 * len(timings) + 99) // 100 - 1)],
        "results": results,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({k: v for k, v in report.items() if k not in ["results", "schema"]}), flush=True)


if __name__ == "__main__":
    main()
