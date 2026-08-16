#!/usr/bin/env bash
# Exclusive ownership helpers for immutable candidate output directories.

BUILD_COMPLETE="${BUILD_COMPLETE:-false}"
OUTPUT_OWNED="${OUTPUT_OWNED:-false}"

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
