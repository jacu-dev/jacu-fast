#!/usr/bin/env python3
"""Static checks for the plugin package. This does not install a plugin."""

import json
import os
import sys
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_URL = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json"


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    sys.exit(1)


def main() -> None:
    root_manifest = load(ROOT / "plugin.json")
    claude_manifest = load(ROOT / ".claude-plugin" / "plugin.json")
    marketplace = load(ROOT / ".claude-plugin" / "marketplace.json")
    if "skills" in root_manifest:
        fail("root plugin.json must not declare skills; the schema rejects it")

    import urllib.request

    with urllib.request.urlopen(SCHEMA_URL, timeout=30) as response:
        schema = json.load(response)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(root_manifest),
        key=lambda item: list(item.path),
    )
    if errors:
        fail("; ".join(error.message for error in errors))

    versions = {
        root_manifest.get("version"),
        claude_manifest.get("version"),
        marketplace["plugins"][0].get("version"),
    }
    if versions != {"0.1.0"}:
        fail(f"expected version 0.1.0 in all three manifests, found {versions}")
    if marketplace["plugins"][0]["source"] != "./":
        fail("marketplace plugin source must stay ./ for this git repository")
    if marketplace["plugins"][0]["name"] != "jacu-fast":
        fail("marketplace plugin name must be jacu-fast")

    skill = (ROOT / "skills" / "jacu-fast" / "SKILL.md").read_text(encoding="utf-8")
    if not skill.startswith("---\n"):
        fail("skill is missing frontmatter")
    frontmatter = skill.split("---", 2)[1]
    if "name: jacu-fast" not in frontmatter:
        fail("skill name must be jacu-fast")
    if "disable-model-invocation: true" not in frontmatter:
        fail("skill must disable model invocation")
    if "bin/jacu" not in skill:
        fail("skill must point at the packaged bin/jacu executable")
    if "contains no Jacu executable" in skill:
        fail("skill still describes a skill-only release")
    runtime = (ROOT / "skills" / "jacu-fast" / "references" / "runtime.md").read_text(encoding="utf-8")
    if "Version 0.1.0 ships `bin/jacu`" not in runtime:
        fail("runtime reference must name the 0.1.0 executable")
    binary = ROOT / "bin" / "jacu"
    if not binary.is_file():
        fail("bin/jacu is missing")
    if not os.access(binary, os.X_OK):
        fail("bin/jacu is not executable")
    magic = binary.read_bytes()[:4]
    if magic not in (b"\xcf\xfa\xed\xfe", b"\xfe\xed\xfa\xcf", b"\xca\xfe\xba\xbe"):
        fail("bin/jacu is not a macOS executable")
    print("PASS: manifests, skill frontmatter, version 0.1.0, and bin/jacu")


if __name__ == "__main__":
    main()
