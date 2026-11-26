#!/usr/bin/env bash
# Backward-compatible entry point; the script lives in build/linux/.
set -euo pipefail

exec "$(dirname "$0")/../build/linux/build.sh" "$@"
