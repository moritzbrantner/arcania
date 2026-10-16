#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

echo "==> Checking Rust formatting"
cargo fmt --all --check

echo "==> Running Rust clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> Running Rust, frontend typecheck and unit tests"
bun run test

echo "==> Building frontend"
bun run --cwd frontend build

echo "==> Running Storybook story tests"
bun run --cwd frontend test:stories

echo "==> Running Playwright e2e tests"
bun run test:e2e

echo "==> Local CI checks passed"
