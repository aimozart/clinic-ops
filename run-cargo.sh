#!/usr/bin/env bash
# Runs cargo in a container (Rust is not installed on this PC). Usage: ./run-cargo.sh test
set -euo pipefail
cd "$(dirname "$0")"
docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/work/target \
  -v "$HOME/.cache/clinic-cargo":/cargo -v "$PWD":/work -w /work rust:1-bookworm cargo "$@"
