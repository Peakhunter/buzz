#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
workflow="$repo_root/.github/workflows/thin-v6-macos-arm64.yml"

python3 - "$workflow" <<'PY'
from __future__ import annotations

import pathlib
import re
import sys

path = pathlib.Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
contract_path = path.parent / "thin-v6-workflow-contract.yml"
if not contract_path.exists():
    raise SystemExit("path-scoped thin-v6 workflow contract is missing")
contract_text = contract_path.read_text(encoding="utf-8")
if "scripts/test-thin-v6-workflow-efficiency.sh" not in contract_text:
    raise SystemExit("path-scoped CI must run the thin-v6 workflow efficiency contract")
for changed_path in (
    ".github/workflows/thin-v6-macos-arm64.yml",
    ".github/workflows/thin-v6-workflow-contract.yml",
    "scripts/test-thin-v6-workflow-efficiency.sh",
):
    if changed_path not in contract_text:
        raise SystemExit(f"path-scoped CI trigger is missing {changed_path}")


def require(pattern: str, message: str, *, flags: int = 0) -> None:
    if re.search(pattern, text, flags) is None:
        raise SystemExit(message)


def reject(pattern: str, message: str, *, flags: int = 0) -> None:
    if re.search(pattern, text, flags) is not None:
        raise SystemExit(message)


on_match = re.search(r"(?ms)^on:\n(?P<body>.*?)(?=^[a-zA-Z][^\n]*:\n)", text)
if on_match is None:
    raise SystemExit("workflow trigger block missing")
on_body = on_match.group("body")
if re.search(r"(?m)^  push:", on_body):
    raise SystemExit("candidate packaging must not run on push")
if not re.search(r"(?m)^  workflow_dispatch:", on_body):
    raise SystemExit("candidate packaging must be manually dispatched")
if not re.search(r"(?ms)^  workflow_dispatch:\n.*?^      source_sha:\n.*?^        required: true$", on_body):
    raise SystemExit("workflow_dispatch must require source_sha")

require(
    r"(?m)^  group: thin-v6-macos-arm64-\$\{\{ inputs\.source_sha \}\}$",
    "concurrency must deduplicate the requested source SHA",
)
require(
    r"(?ms)^  verify_source:\n.*?^[ ]{6}- name: Resolve immutable source SHA\n.*?\[\[ \"\$SOURCE_SHA\" =~ \^\[0-9a-fA-F\]\{40\}\$ \]\]",
    "a preflight job must reject non-full source identities",
)
require(
    r"(?ms)^  verify_source:\n.*?actions/workflows/ci\.yml/runs.*?head_sha",
    "preflight must verify source CI for the exact SHA",
)
require(
    r"(?ms)^  build:\n.*?^    needs: verify_source$",
    "the costly build must depend on source verification",
)
require(
    r"(?ms)^      - name: Check out exact candidate source\n.*?^          ref: \$\{\{ needs\.verify_source\.outputs\.source_sha \}\}$",
    "the build must check out the verified immutable SHA",
)
reject(
    r"\$\{\{ github\.sha \}\}",
    "github.sha must not substitute for the explicitly requested source SHA",
)

require(
    r"(?ms)^      - name: Restore Cargo dependency cache\n.*?^          path: \|\n            ~/\.cargo/registry\n            ~/\.cargo/git\n",
    "Cargo downloads must be cached separately from compiler outputs",
)
require(
    r"(?ms)^      - name: Restore source-sensitive Cargo target cache\n.*?^          key: .*needs\.verify_source\.outputs\.source_sha.*?^          restore-keys: \|",
    "compiler outputs need an exact source key plus compatible restore prefix",
)
require(
    r"(?m)^        if: success\(\) && steps\.cargo-target-restore\.outputs\.cache-hit != 'true'$",
    "Cargo target cache may be saved only after a successful build",
)
reject(
    r"(?m)^        if: always\(\).*cache-hit",
    "failed or cancelled runs must not save caches",
)
require(
    r"(?m)^      - name: Publish cache telemetry$",
    "candidate workflow must expose cache hit/miss telemetry",
)
require(
    r"GITHUB_STEP_SUMMARY",
    "cache telemetry must be visible in the workflow summary",
)

print("thin-v6 workflow efficiency contract passed")
PY
