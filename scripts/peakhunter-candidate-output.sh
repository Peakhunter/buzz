#!/usr/bin/env bash
# Exclusive ownership helpers for immutable candidate output directories.

# These flags are process-internal proof state. Never trust or export inherited
# values: a poisoned OUTPUT_OWNED could authorize deleting an unclaimed path,
# while a poisoned BUILD_COMPLETE could suppress cleanup of an incomplete build.
unset BUILD_COMPLETE OUTPUT_OWNED
BUILD_COMPLETE=false
OUTPUT_OWNED=false

claim_output_root() {
  local parent
  parent="$(dirname "$OUTPUT_ROOT")"
  mkdir -p -- "$parent"
  if ! mkdir -- "$OUTPUT_ROOT"; then
    echo "refusing to overwrite existing immutable candidate: $OUTPUT_ROOT" >&2
    return 1
  fi
  OUTPUT_OWNED=true
}

cleanup_output_root() {
  if [[ "$OUTPUT_OWNED" == true && "$BUILD_COMPLETE" != true ]]; then
    rm -rf -- "$OUTPUT_ROOT"
    OUTPUT_OWNED=false
  fi
}
