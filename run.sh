#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

exec cargo run --quiet --release --locked \
    --manifest-path "$script_dir/Cargo.toml" -- "$@"
