#!/usr/bin/env bash
# Regenerate the TS API bindings under packages/xlsx-preview/src/api-schema/
# from the shared Rust DTOs in crates/xlcore-types.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

cargo test -p xlcore-types --features typescript export_bindings

cd packages/xlsx-preview
pnpm exec biome format --write src/api-schema/ >/dev/null
