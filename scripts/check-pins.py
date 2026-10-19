#!/usr/bin/env python3
"""Offline supply-chain pin check (standard library only; no network).

Fails on:
  * a Dockerfile FROM / COPY --from image, or a YAML `image:` / `container:` value,
    that is not `name:tag@sha256:<64 hex>` on one line (Renovate-compatible);
  * a known third-party image reference in scripts/ or infra/ without that pin;
  * a GitHub Actions `uses:` that is not a 40-hex commit SHA.

Exempt: locally built images (`*:local`, `${VAR:-name:local}`), Dockerfile stage
aliases, `scratch`, and local `./` actions. The check never resolves a digest, so
it cannot tell whether a pin is current: refresh pins with
`docker buildx imagetools inspect NAME:TAG` and update the tag and digest together.
"""
import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Repositories that scripts start containers from. Names found in Dockerfile FROM
# lines and YAML `image:` values are added automatically (collect_names).
THIRD_PARTY_IMAGES = frozenset({
    "alpine", "busybox", "caddy", "debian", "ghcr.io/dexidp/dex", "hashicorp/vault", "neo4j",
    "node", "pgvector/pgvector", "postgres", "python", "redis", "rust", "ubuntu",
})
# `node:fs` and similar are Node.js module specifiers, not node image tags.
NODE_BUILTINS = frozenset({
    "assert", "buffer", "child_process", "cluster", "crypto", "dns", "events", "fs", "http", "https",
    "net", "os", "path", "perf_hooks", "process", "readline", "stream", "test", "timers", "tty",
    "url", "util", "v8", "vm", "worker_threads", "zlib",
})
PRUNED = frozenset({"node_modules", "target", "references", "dist", "__pycache__"})
SKIPPED_FROM_SCRIPT_SCAN = ("scripts/tests/", "scripts/check-pins.py")
MAX_BYTES = 2 * 1024 * 1024

DIGEST = r"@sha256:[0-9a-f]{64}"
PINNED = re.compile(rf"^(?P<name>[^@\s]+?):(?P<tag>[A-Za-z0-9_][A-Za-z0-9._-]{{0,127}}){DIGEST}$")
LOCAL = re.compile(r":local\}?$")
COMMIT = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+(/[^@\s]+)?@[0-9a-f]{40}$")
IMAGE_KEY = re.compile(r"^\s*(?:-\s+)?(?:image|container):\s*(.*?)\s*$")
USES_KEY = re.compile(r"^\s*(?:-\s+)?uses:\s*(.*?)\s*$")
FROM_FLAG = re.compile(r"--from=(\S+)")


def unquote(value):
    """Return the scalar of a single-line YAML value, without quotes or comment."""
    value = value.strip()
    if value and value[0] in "'\"":
        end = value.find(value[0], 1)
        return value[1:end] if end > 0 else value[1:]
    return re.split(r"\s+#", value, maxsplit=1)[0].strip()


def image_problem(reference):
    """Return why `reference` is not an acceptable image pin, or None."""
    if LOCAL.search(reference) or PINNED.match(reference):
        return None
    if "@sha256:" in reference:
        return f"{reference} must be name:tag@sha256:<64 hex> on one line (tag missing or digest malformed)"
    return f"{reference} has no @sha256 digest pin (use name:tag@sha256:<index digest>)"


def logical_lines(text):
    """Yield (line number, instruction) with backslash continuations joined."""
    buffer, start = "", 0
    for number, raw in enumerate(text.splitlines(), 1):
        if raw.strip().startswith("#") or (not buffer and not raw.strip()):
            continue
        if not buffer:
            start = number
        if raw.rstrip().endswith("\\"):
            buffer += raw.rstrip()[:-1] + " "
            continue
        yield start, buffer + raw
        buffer = ""
    if buffer:
        yield start, buffer


def check_dockerfile(text):
    """Return (references checked, [(line, message)]) for one Dockerfile."""
    problems, checked, stages = [], 0, set()
    for number, line in logical_lines(text):
        parts = line.split(None, 1)
        instruction = parts[0].upper()
        rest = parts[1] if len(parts) > 1 else ""
        if instruction == "FROM":
            tokens = [t for t in rest.split() if not t.startswith("--")]
            if not tokens:
                continue
            reference = tokens[0]
            if reference.lower() != "scratch" and reference.lower() not in stages:
                checked += 1
                if (problem := image_problem(reference)):
                    problems.append((number, "FROM " + problem))
            if len(tokens) >= 3 and tokens[1].upper() == "AS":
                stages.add(tokens[2].lower())
        elif instruction in ("COPY", "ADD") and (match := FROM_FLAG.search(rest)):
            reference = match.group(1)
            if not reference.isdigit() and reference.lower() not in stages:
                checked += 1
                if (problem := image_problem(reference)):
                    problems.append((number, "--from=" + problem))
    return checked, problems


def check_yaml(text, actions):
    """Return (references checked, [(line, message)]) for one YAML file.

    `actions` also enforces commit-SHA pins on `uses:` (files under .github/).
    """
    problems, checked = [], 0
    for number, line in enumerate(text.splitlines(), 1):
        if line.lstrip().startswith("#"):
            continue
        if (match := IMAGE_KEY.match(line)) and (value := unquote(match.group(1))) \
                and value[0] not in "|>&*":
            checked += 1
            if (problem := image_problem(value)):
                problems.append((number, "image: " + problem))
        elif actions and (match := USES_KEY.match(line)) and (value := unquote(match.group(1))):
            if value.startswith(("./", "../")):
                continue
            checked += 1
            if value.startswith("docker://"):
                if (problem := image_problem(value.removeprefix("docker://"))):
                    problems.append((number, "uses: docker://" + problem))
            elif not COMMIT.match(value):
                problems.append((number, f"uses: {value} must pin a 40-hex commit SHA (owner/repo@<sha> # tag)"))
    return checked, problems


def image_name(reference):
    """Repository part of an image reference (no tag, digest or docker.io prefix)."""
    name = reference.split("@", 1)[0]
    name = name.rsplit(":", 1)[0] if ":" in name.rsplit("/", 1)[-1] else name
    return name.removeprefix("docker.io/").removeprefix("library/")


def collect_names(documents):
    """Image repositories used by Dockerfiles and YAML, for the script scan."""
    names = set()
    for kind, text in documents:
        stages = set()
        if kind == "dockerfile":
            for _, line in logical_lines(text):
                parts = line.split(None, 1)
                if parts[0].upper() != "FROM" or len(parts) < 2:
                    continue
                tokens = [t for t in parts[1].split() if not t.startswith("--")]
                if tokens and tokens[0].lower() != "scratch" and tokens[0].lower() not in stages \
                        and "$" not in tokens[0] and not LOCAL.search(tokens[0]):
                    names.add(image_name(tokens[0]))
                if len(tokens) >= 3 and tokens[1].upper() == "AS":
                    stages.add(tokens[2].lower())
        else:
            for line in text.splitlines():
                if (match := IMAGE_KEY.match(line)) and (value := unquote(match.group(1))) \
                        and "$" not in value and not LOCAL.search(value) and value[0] not in "|>&*":
                    names.add(image_name(value))
    return names


def script_pattern(names):
    alternatives = "|".join(re.escape(n) for n in sorted(names, key=len, reverse=True))
    return re.compile(
        rf"(?<![\w./@:-])(?:docker\.io/(?:library/)?)?(?P<name>{alternatives}):"
        rf"(?P<tag>[A-Za-z0-9_][A-Za-z0-9._-]*)(?P<digest>{DIGEST})?")


def check_script(text, pattern):
    """Return (references checked, [(line, message)]) for a shell/Python script."""
    problems, checked = [], 0
    for number, line in enumerate(text.splitlines(), 1):
        for match in pattern.finditer(line):
            if match.group("name") == "node" and match.group("tag") in NODE_BUILTINS:
                continue
            checked += 1
            if not match.group("digest"):
                problems.append((number, f"{match.group('name')}:{match.group('tag')} has no "
                                         "@sha256 digest pin (use name:tag@sha256:<index digest>)"))
    return checked, problems


def hidden_or_pruned(directory):
    return directory in PRUNED or (directory.startswith(".") and directory != ".github")


def listed_files(root):
    """Relative posix paths: Git's tracked and unignored files, else a directory walk."""
    try:
        listing = subprocess.run(
            ["git", "-C", str(root), "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
            capture_output=True, check=True, timeout=30).stdout.decode()
        return sorted(path for path in listing.split("\0") if path)
    except (OSError, subprocess.SubprocessError, UnicodeDecodeError):
        pass
    found = []
    for directory, names, files in os.walk(root):
        names[:] = [n for n in names if not hidden_or_pruned(n)]
        found.extend(Path(directory, name).relative_to(root).as_posix() for name in files)
    return sorted(found)


def discover(root):
    """Yield (relative posix path, kind) for files this check reads."""
    for relative in listed_files(root):
        *directories, name = relative.split("/")
        if any(hidden_or_pruned(d) for d in directories):
            continue
        if name == "Dockerfile" or name.startswith("Dockerfile.") or name.endswith(".Dockerfile") \
                or name == "Containerfile":
            kind = "dockerfile"
        elif name.endswith((".yaml", ".yml")):
            kind = "yaml"
        elif relative.startswith(("scripts/", "infra/")) \
                and not relative.startswith(SKIPPED_FROM_SCRIPT_SCAN):
            kind = "script"
        else:
            continue
        yield relative, kind


def read_text(path):
    try:
        if path.is_symlink() or path.stat().st_size > MAX_BYTES:
            return None
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None


def check_tree(root):
    """Return ({kind: references checked}, [(path, line, message)])."""
    root = Path(root)
    documents = {}
    for relative, kind in discover(root):
        if (text := read_text(root / relative)) is not None:
            documents[relative] = (kind, text)
    pattern = script_pattern(THIRD_PARTY_IMAGES | collect_names(
        [(k, t) for k, t in documents.values() if k != "script"]))
    counts = {"dockerfile": 0, "yaml": 0, "script": 0}
    findings = []
    for relative, (kind, text) in sorted(documents.items()):
        if kind == "dockerfile":
            checked, problems = check_dockerfile(text)
        elif kind == "yaml":
            checked, problems = check_yaml(text, actions=relative.startswith(".github/"))
        else:
            checked, problems = check_script(text, pattern)
        counts[kind] += checked
        findings.extend((relative, number, message) for number, message in problems)
    return counts, findings


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--root", type=Path, default=ROOT, help="repository root to scan")
    options = parser.parse_args(argv)
    counts, findings = check_tree(options.root.resolve())
    for relative, number, message in findings:
        print(f"{relative}:{number}: {message}", file=sys.stderr)
    summary = ", ".join(f"{count} {kind}" for kind, count in counts.items())
    if findings:
        print(f"check-pins: {len(findings)} unpinned reference(s) ({summary} references checked)", file=sys.stderr)
        return 1
    print(f"check-pins: OK ({summary} references checked)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
