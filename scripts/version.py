#!/usr/bin/env python3
"""Developer-only version synchronization; Python 3 is not a plugin runtime dependency."""
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?")


def process(new=None):
    if new is not None and not VERSION.fullmatch(new):
        raise ValueError("expected a release version such as 0.1.0 or 0.1.0-rc1")
    mapping = json.loads((ROOT / ".version-bump.json").read_text())
    canonical = json.loads((ROOT / mapping["canonical"]["file"]).read_text())[mapping["canonical"]["field"]]
    updates = []
    for entry in mapping["files"]:
        path = ROOT / entry["file"]
        original = path.read_text()
        if entry.get("format") == "toml":
            # The package manifest has one standalone version field; dependency
            # inline tables are deliberately excluded by anchoring the line.
            matches = list(re.finditer(r'^version = "([^"]+)"$', original, re.M))
            if len(matches) != 1 or entry["field"] != "package.version":
                raise ValueError("ambiguous Cargo version field")
            found = matches[0].group(1)
            replacement = original[:matches[0].start(1)] + (new or found) + original[matches[0].end(1):]
        else:
            document = json.loads(original)
            parts = entry["field"].split(".")
            node = document
            for part in parts[:-1]:
                node = node[int(part)] if isinstance(node, list) else node[part]
            found = node[parts[-1]]
            node[parts[-1]] = new or found
            replacement = json.dumps(document, indent=2) + "\n"
        if new is None and found != canonical:
            raise ValueError(f"version mismatch in {entry['file']}")
        updates.append((path, replacement))
    lock = ROOT / "Cargo.lock"
    lock_text = lock.read_text()
    pattern = r'(\[\[package\]\]\nname = "buzz-kit"\nversion = ")([^"]+)(")'
    match = re.search(pattern, lock_text)
    if not match or (new is None and match.group(2) != canonical):
        raise ValueError("Cargo.lock package version mismatch")
    if new is not None:
        updates.append((lock, re.sub(pattern, lambda m: m.group(1) + new + m.group(3), lock_text)))
        # Validate every input before changing any file. No resolution or network.
        for path, content in updates:
            path.write_text(content)
    print(f"Versions agree: {new or canonical}")


if __name__ == "__main__":
    try:
        if len(sys.argv) not in (1, 2):
            raise ValueError("usage: check-version | bump-version VERSION")
        process(sys.argv[1] if len(sys.argv) == 2 else None)
    except (ValueError, KeyError, OSError) as error:
        sys.exit(str(error))
