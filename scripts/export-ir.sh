#!/usr/bin/env bash
set -Eeuo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/export-ir.sh [cue/examples/name.cue ...]

Export CUE example definitions to normalized JSON IR without involving Rust.
The exported JSON is validated against cue/schema #IR when schema files are included.

Defaults:
  - input examples: cue/examples/*.cue
  - output directory: generated/examples
  - optional schema files: cue/schema/*.cue, when that directory exists

Environment:
  CUE_EXAMPLES_DIR     Override the examples directory. Default: cue/examples
  CUE_EXAMPLE_GLOB     Override the example filename glob. Default: *.cue
  CUE_SCHEMA_DIR       Override the optional schema directory. Default: cue/schema
  CUE_INCLUDE_SCHEMA   Set to false/0/no to avoid prepending schema files
  CUE_EXPORT_EXPR      Optional CUE expression to export, for example: scenario
  GENERATED_EXAMPLES_DIR Override the output directory. Default: generated/examples
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
  echo "[cue-export] error: CUE CLI not found. Install it first: https://cuelang.org/docs/introduction/installation/" >&2
  exit 127
fi

examples_dir="${CUE_EXAMPLES_DIR:-cue/examples}"
example_glob="${CUE_EXAMPLE_GLOB:-*.cue}"
schema_dir="${CUE_SCHEMA_DIR:-cue/schema}"
include_schema="${CUE_INCLUDE_SCHEMA:-true}"
export_expr="${CUE_EXPORT_EXPR:-}"
out_dir="${GENERATED_EXAMPLES_DIR:-generated/examples}"

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

mkdir -p "$out_dir"

if ((${#examples[@]} == 0)); then
  echo "[cue-export] no CUE examples found under ${examples_dir}/${example_glob}; nothing to export."
  exit 0
fi

for example in "${examples[@]}"; do
  if [[ ! -f "$example" ]]; then
    echo "[cue-export] error: example file not found: $example" >&2
    exit 1
  fi

  filename="$(basename "$example")"
  stem="${filename%.cue}"
  out_file="$out_dir/${stem}.json"
  tmp_file="$out_file.tmp.$$.json"

  cmd=(cue export -E --out json --outfile "$tmp_file")
  if [[ -n "$export_expr" ]]; then
    cmd+=(-e "$export_expr")
  fi
  cmd+=("${schema_files[@]}" "$example")

  echo "[cue-export] exporting $example -> $out_file"
  print_cmd "${cmd[@]}"
  if "${cmd[@]}"; then
    if ((${#schema_files[@]} > 0)); then
      vet_cmd=(cue vet -E -c -d '#IR' "${schema_files[@]}" "$tmp_file")
      echo "[cue-export] validating exported IR against closed #IR schema"
      print_cmd "${vet_cmd[@]}"
      if ! "${vet_cmd[@]}"; then
        rm -f "$tmp_file"
        exit 1
      fi
    fi
    mv "$tmp_file" "$out_file"
  else
    rm -f "$tmp_file"
    exit 1
  fi
done

echo "[cue-export] exported ${#examples[@]} example(s) to $out_dir."
