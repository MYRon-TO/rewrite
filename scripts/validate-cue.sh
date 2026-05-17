#!/usr/bin/env bash
set -Eeuo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/validate-cue.sh [cue/examples/name.cue ...]

Validate CUE example definitions without involving Rust.

Defaults:
  - input examples: cue/examples/*.cue
  - optional schema files: cue/schema/*.cue, when that directory exists

Environment:
  CUE_EXAMPLES_DIR       Override the examples directory. Default: cue/examples
  CUE_EXAMPLE_GLOB       Override the example filename glob. Default: *.cue
  CUE_SCHEMA_DIR         Override the optional schema directory. Default: cue/schema
  CUE_INCLUDE_SCHEMA     Set to false/0/no to avoid prepending schema files
  CUE_REQUIRE_CONCRETE   Set to false/0/no to allow incomplete regular fields
USAGE
}

is_false() {
  case "${1:-}" in
    false|FALSE|0|no|NO) return 0 ;;
    *) return 1 ;;
  esac
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
  echo "[cue-validate] error: CUE CLI not found. Install it first: https://cuelang.org/docs/introduction/installation/" >&2
  exit 127
fi

examples_dir="${CUE_EXAMPLES_DIR:-cue/examples}"
example_glob="${CUE_EXAMPLE_GLOB:-*.cue}"
schema_dir="${CUE_SCHEMA_DIR:-cue/schema}"
include_schema="${CUE_INCLUDE_SCHEMA:-true}"
require_concrete="${CUE_REQUIRE_CONCRETE:-true}"

schema_files=()
if ! is_false "$include_schema" && [[ -n "$schema_dir" && -d "$schema_dir" ]]; then
  mapfile -t schema_files < <(find "$schema_dir" -maxdepth 1 -type f -name '*.cue' -print | sort)
fi

examples=()
if (($# > 0)); then
  examples=("$@")
else
  if [[ -d "$examples_dir" ]]; then
    mapfile -t examples < <(find "$examples_dir" -maxdepth 1 -type f -name "$example_glob" -print | sort)
  fi
fi

if ((${#examples[@]} == 0)); then
  echo "[cue-validate] no CUE examples found under ${examples_dir}/${example_glob}; nothing to validate."
  exit 0
fi

for example in "${examples[@]}"; do
  if [[ ! -f "$example" ]]; then
    echo "[cue-validate] error: example file not found: $example" >&2
    exit 1
  fi

  cmd=(cue vet -E)
  if is_false "$require_concrete"; then
    cmd+=(-c=false)
  else
    cmd+=(-c)
  fi
  cmd+=("${schema_files[@]}" "$example")

  echo "[cue-validate] validating $example"
  print_cmd "${cmd[@]}"
  "${cmd[@]}"
done

echo "[cue-validate] validated ${#examples[@]} example(s)."
