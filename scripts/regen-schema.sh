#!/usr/bin/env bash
# Regenerate the TS schema bindings under packages/xlsx-preview/src/schema/
# from the Rust types in crates/xlcore-export/src/schema*.rs.
#
# ts-rs emits one .ts file per #[ts(export)] type. The committed files are
# additionally biome-formatted, so we run biome over them afterwards.
#
# CI uses this same flow plus `git diff --exit-code` to guard against drift
# between the Rust schema and the checked-in TS bindings.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

cargo test --release -p xlcore-export --features typescript export_bindings

cd packages/xlsx-preview
pnpm exec biome format --write src/schema/ >/dev/null
