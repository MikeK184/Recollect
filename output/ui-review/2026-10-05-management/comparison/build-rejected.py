"""Build a portable visual comparison from unmodified source image bytes."""
import base64
import hashlib
import json
import struct
from pathlib import Path

HERE = Path(__file__).resolve().parent
REVIEW = HERE.parent
REPO = HERE.parents[3]
CONCEPTS = REPO / "output/imagegen/2026-10-05-management-concepts-01a10b36"
ASSETS = {}


def dimensions(raw):
    if raw.startswith(b"\x89PNG"):
        return struct.unpack(">II", raw[16:24])
    offset = 2
    while offset < len(raw):
        if raw[offset] != 255:
            offset += 1
            continue
        marker = raw[offset + 1]
        offset += 2
        if marker in (0xD8, 0xD9):
            continue
        length = int.from_bytes(raw[offset:offset + 2], "big")
        if marker in (0xC0, 0xC1, 0xC2):
            height, width = struct.unpack(">HH", raw[offset + 3:offset + 7])
            return width, height
        offset += length
    raise ValueError("Unsupported image dimensions")


def asset(path):
    path = path.resolve()
    key = str(path.relative_to(REPO))
    if key not in ASSETS:
        raw = path.read_bytes()
        width, height = dimensions(raw)
        mime = "image/png" if path.suffix == ".png" else "image/jpeg"
        ASSETS[key] = {
            "src": f"data:{mime};base64," + base64.b64encode(raw).decode(),
            "width": width,
            "height": height,
            "file": path.name,
            "path": key,
            "sha256": hashlib.sha256(raw).hexdigest(),
        }
    return key


def original(name, label):
    return {"asset": asset(REVIEW / "before" / (name + ".png")), "label": label}


def vision(name, label="Approved generated concept"):
    return {"asset": asset(CONCEPTS / (name + ".png")), "label": label}


def after(name, fresh=None, label=None):
    images = {str(size): asset(REVIEW / "after-final" / f"{name}-{size}.jpg") for size in (1440, 1920)}
    if fresh:
        images["1600"] = asset(REVIEW / "after-comparison" / (fresh + "-1600.jpg"))
    return {"label": label or name, "images": images}


PAGES = [
    {
        "id": "connections", "title": "Connections", "number": "01",
        "intent": "A composed connection list with a useful adjacent inspector.",
        "before": [original("01-edit-connection", "Original edit dialog · overview visible behind")],
        "vision": [vision("01-connections")],
        "after": [
            after("01-connections", "01-connections", "Overview + inspector"),
            after("01-connections-tools", label="Inspector · Tools tab"),
            after("02-edit-top", "02-edit", "Edit dialog · top"),
            after("02-edit-bottom", label="Edit dialog · bottom"),
        ],
        "gaps": [
            "The vision anchors the inspector beside the page heading. The implementation puts it below the full-width tabs and toolbar, leaving a much larger empty list area.",
            "The heading, icon tiles, text contrast and status pills are smaller and quieter. The Add connection action moved from the header into the toolbar.",
            "The concept illustrates three connections; the real Brain has one. That inventory difference is separate from the composition and typography gaps.",
        ],
    },
    {
        "id": "add-connection", "title": "Add connection", "number": "02",
        "intent": "One polished setup surface for URL, pasted configuration and credentials.",
        "before": [original("02-add-url", "Original URL-only setup"), original("02-add-approved", "Original five-step approved connector wizard")],
        "vision": [vision("02-add-connection")],
        "after": [
            after("02-add-config-parsed", "02-add-config", "Paste config · parsed example; secret unset"),
            after("02-add-url", "02-add-url", "Server URL · empty form"),
            after("02-add-approved-top", label="Approved connector · top"),
            after("02-add-approved-bottom", label="Approved connector · bottom"),
        ],
        "gaps": [
            "The vision uses a broad, tightly grouped dialog. The implementation has a smaller relative width, larger vertical gaps and a scrollbar even at 1600 × 1000.",
            "The format tabs, numbered syntax-highlighted editor, success strip and horizontal credential controls are replaced by a dropdown, plain textarea, sentence and stacked fields.",
            "The compact execution/environment summary, optional connection name and Review connection footer from the vision are absent from this parsed state.",
        ],
    },
    {
        "id": "connectors", "title": "Global connectors", "number": "03",
        "intent": "An installation-level library with deliberate catalogue and import layouts.",
        "before": [original("02-add-approved", "Previously: connector selection inside a Brain wizard")],
        "beforeNote": "There was no installation-level Connectors page. This original screenshot shows where approved connectors were selected before.",
        "vision": [vision("03-global-connectors-v2"), vision("03-global-connectors", "Initial generated variant · superseded")],
        "after": [
            after("03-connectors", "03-connectors", "Installation connector library"),
            after("03-connectors-no-match", label="Search · no matches"),
        ],
        "gaps": [
            "The vision uses compact catalogue cards beside a prominent import/review/approve panel. The implementation has a wide single card and a distant narrow helper panel.",
            "The colourful icon tiles, transport chips, approval tabs and structured import steps were simplified; the resulting visual hierarchy is substantially weaker.",
            "The concept illustrates six definitions; the real installation has one. No additional definitions were invented for this comparison.",
        ],
    },
    {
        "id": "privacy", "title": "Privacy", "number": "04",
        "intent": "Compact, stable inline settings with the key sections visible together.",
        "before": [original("04-privacy-edit", "Original retention edit state · scrolled page")],
        "vision": [vision("04-privacy")],
        "after": [
            after("04-privacy-edit", "04-privacy-edit", "Retention edit · page auto-scrolled to editor"),
            after("04-privacy-view", "04-privacy-view", "Read view · top of page"),
        ],
        "gaps": [
            "The vision fits retention, capture, evidence and repository controls into a compact viewport. The implementation uses much taller sections; later settings fall below the fold.",
            "Capture remains summary prose rather than the pictured switches. Retention follows Capture, and evidence is expanded into several separate rows.",
            "Inline inputs now exist, but section rhythm, compact units, notice placement and action styling differ. Opening retention also scrolls the current page away from its heading.",
        ],
    },
    {
        "id": "ai-permissions", "title": "AI permissions", "number": "05",
        "intent": "A clear permission policy with calm coverage and model information beside it.",
        "before": [original("05-ai-permissions", "Original AI policy + expanded diagnostics")],
        "vision": [vision("05-ai-permissions-v2"), vision("05-ai-permissions", "Initial generated variant · superseded")],
        "after": [
            after("05-ai", "05-ai", "Policy + search coverage"),
            after("05-ai-diagnostics", label="Expanded model diagnostics"),
        ],
        "gaps": [
            "The broad two-column structure is present, but the provider identity row and prominent Automatic memory strip from the vision are missing from the policy card.",
            "Permission helpers sit beside labels rather than beneath them, compressing the hierarchy. Allowed-content chips use three columns instead of two.",
            "The vision separates Search coverage, Installed models and More options. The implementation combines provider/models with a single diagnostics link and adds a large amber warning. Counts differ because these are real captures taken at different times.",
        ],
    },
    {
        "id": "agents", "title": "Agents", "number": "06",
        "intent": "A legible roster and two focused companion cards, with useful visual weight.",
        "before": [original("06-agents", "Original attached Agents screenshot · content only")],
        "vision": [vision("06-agents")],
        "after": [after("06-agents", "06-agents", "Observed roster + setup + memory check")],
        "gaps": [
            "The roster and two companion cards exist, but names, icons and status pills have much less contrast and visual weight than the vision.",
            "The concept has an owner avatar, recognisable host tiles, compact observed dates and a roster footer action. The implementation uses generic tiles, full timestamps and a separate My agents link.",
            "The setup card is instruction-heavy and uses disclosures and a full-width copy action. It does not reproduce the concise command surface and small setup link shown in the concept.",
        ],
    },
]

template = (HERE / "template.html").read_text()
data = json.dumps({"pages": PAGES, "assets": ASSETS}, separators=(",", ":")).replace("<", "\\u003c")
output = HERE / "before-vision-after.html"
output.write_text(template.replace("__COMPARISON_DATA__", data))
(HERE / "manifest.json").write_text(json.dumps({"pages": PAGES, "assets": {key: {k: v for k, v in value.items() if k != "src"} for key, value in ASSETS.items()}}, indent=2) + "\n")
print(f"Built {output}: {len(PAGES)} pages, {len(ASSETS)} unmodified images, {output.stat().st_size / 1024 / 1024:.1f} MiB")
