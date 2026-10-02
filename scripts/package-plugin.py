#!/usr/bin/env python3
"""Build a self-contained native plugin; consumers need neither Cargo nor a companion."""
import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--output", type=Path)
parser.add_argument("--debug", action="store_true", help="Package the development build instead of the optimized runtime")
parser.add_argument("--skip-build", action="store_true", help="Use an already built runtime")
args = parser.parse_args()
target = platform.system().lower() + "-" + platform.machine().lower()
output = (args.output or ROOT / ".cache" / "plugin-dist" / target).resolve()
if not output.is_relative_to(ROOT / ".cache"):
    parser.error("Build output must stay inside this repository's .cache directory")
if output.exists():
    parser.error("Choose a fresh output directory; existing packages are never overwritten")
profile = "debug" if args.debug else "release"
if not args.skip_build:
    subprocess.run(["cargo", "build", "--locked", "-p", "recollect-agent", "--bin", "recollect-plugin"] + ([] if args.debug else ["--release"]), cwd=ROOT, check=True)
binary = ROOT / "target" / profile / ("recollect-plugin.exe" if os.name == "nt" else "recollect-plugin")
if not binary.is_file():
    parser.error("Build the plugin runtime first")
subprocess.run(["python3", str(ROOT / "scripts/setup-enola.py")], cwd=ROOT, check=True)
extractor = ROOT / ".cache/enola-tools/v0.4.19"
source = ROOT / "plugins" / "recollect"
shutil.copytree(source, output, ignore=shutil.ignore_patterns("node_modules", "bin", "*.tgz"))
plugin = output / "plugins" / "recollect-memory"
(plugin / "bin").mkdir()
shutil.copy2(binary, plugin / "bin" / binary.name)
os.chmod(plugin / "bin" / binary.name, 0o755)
shutil.copy2(extractor / "enola", plugin / "bin/enola")
os.chmod(plugin / "bin/enola", 0o755)
notices = plugin / "third-party/enola"
notices.mkdir(parents=True)
for name in ("LICENSE", "NOTICE"):
    shutil.copy2(extractor / name, notices / name)
(output / "build.json").write_text(json.dumps({"platform":target,"runtime":"bundled","profile":profile,"extractor":"enola-0.4.19","runner":"opt-in --with-runner"}, indent=2)+"\n")
print(output)
