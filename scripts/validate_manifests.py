#!/usr/bin/env python3
"""Static checks for the skill-only package. This does not install a plugin."""

import json
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
    if versions != {"0.0.2"}:
        fail(f"expected version 0.0.2 in all three manifests, found {versions}")
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
    if "contains no Jacu executable" not in skill:
        fail("skill must state that this release has no executable")
    for forbidden in ("Use `prepare`", "Use `verify`", "Use `report`"):
        if forbidden in skill:
            fail(f"skill still requires a future command: {forbidden}")
    runtime = ROOT / "skills" / "jacu-fast" / "references" / "runtime.md"
    if "does not ship a `jacu` executable" not in runtime.read_text(encoding="utf-8"):
        fail("runtime reference must say the executable is not shipped")
    print("PASS: manifests, skill frontmatter, and version 0.0.2")


if __name__ == "__main__":
    main()
