#!/usr/bin/env bash
set -Eeuo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/validate-invalid-cue.sh [cue/examples/invalid/name.cue ...]

Assert that invalid CUE examples are rejected by the schema. Each provided file
must fail validation; any unexpected pass fails this script.

Defaults:
  - input examples: cue/examples/invalid/*.cue
  - schema files: cue/schema/*.cue
USAGE
}

print_cmd() {
  printf '  '
  printf '%q ' "$@"
  printf '\n'
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v cue >/dev/null 2>&1; then
  echo "[cue-invalid] error: CUE CLI not found. Install it first: https://cuelang.org/docs/introduction/installation/" >&2
  exit 127
fi

invalid_dir="${CUE_INVALID_EXAMPLES_DIR:-cue/examples/invalid}"
schema_dir="${CUE_SCHEMA_DIR:-cue/schema}"

schema_files=()
if [[ -d "$schema_dir" ]]; then
  mapfile -t schema_files < <(find "$schema_dir" -maxdepth 1 -type f -name '*.cue' -print | sort)
fi

if ((${#schema_files[@]} == 0)); then
  echo "[cue-invalid] error: no CUE schema files found under $schema_dir" >&2
  exit 1
fi

examples=()
if (($# > 0)); then
  examples=("$@")
else
  if [[ -d "$invalid_dir" ]]; then
    mapfile -t examples < <(find "$invalid_dir" -maxdepth 1 -type f -name '*.cue' -print | sort)
  fi
fi

if ((${#examples[@]} == 0)); then
  echo "[cue-invalid] no invalid CUE examples found under ${invalid_dir}/*.cue; nothing to validate."
  exit 0
fi

unexpected_passes=0
for example in "${examples[@]}"; do
  if [[ ! -f "$example" ]]; then
    echo "[cue-invalid] error: example file not found: $example" >&2
    exit 1
  fi

  cmd=(cue vet -E -c -d '#IR')
  cmd+=("${schema_files[@]}" "$example")

  echo "[cue-invalid] expecting validation failure for $example"
  print_cmd "${cmd[@]}"

  tmp_out="$(mktemp)"
  tmp_err="$(mktemp)"
  if "${cmd[@]}" >"$tmp_out" 2>"$tmp_err"; then
    echo "[cue-invalid] unexpected pass: $example" >&2
    unexpected_passes=$((unexpected_passes + 1))
  else
    echo "[cue-invalid] rejected as expected: $example"
  fi
  rm -f "$tmp_out" "$tmp_err"
done

if ((unexpected_passes > 0)); then
  echo "[cue-invalid] $unexpected_passes invalid example(s) unexpectedly passed." >&2
  exit 1
fi

echo "[cue-invalid] all ${#examples[@]} invalid example(s) were rejected."
