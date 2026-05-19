SHELL := /usr/bin/env bash

.DEFAULT_GOAL := help

CARGO ?= cargo
CUE_VALIDATE ?= scripts/validate-cue.sh
CUE_EXPORT ?= scripts/export-ir.sh
CUE_INVALID ?= scripts/validate-invalid-cue.sh
EXAMPLE_IR ?= generated/examples/ssh_bruteforce.json
RUN_OUT ?= out/events.jsonl
SEED ?= 42

.PHONY: help check ci rust-check test cue-validate cue-invalid cue-export run-example clean-generated

help: ## Show available make targets.
	@awk 'BEGIN {FS = ":.*##"; printf "Available targets:\n"} /^[a-zA-Z0-9_.-]+:.*##/ {printf "  %-18s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

check: cue-validate cue-invalid cue-export rust-check test ## Run the local quality gate: CUE validation, invalid CUE rejection, IR export, cargo check, and tests.

ci: check ## CI entrypoint; kept as an alias so CI can run one stable command.

rust-check: ## Run Rust compile-time checks.
	$(CARGO) check

test: ## Run Rust tests.
	$(CARGO) test

cue-validate: ## Validate CUE examples without involving Rust.
	$(CUE_VALIDATE)

cue-invalid: ## Assert invalid CUE examples are rejected by schema validation.
	$(CUE_INVALID)

cue-export: ## Export CUE examples to generated JSON IR without involving Rust.
	$(CUE_EXPORT)

run-example: cue-export ## Run the default generated example when the CLI supports generation.
	@if [[ ! -f "$(EXAMPLE_IR)" ]]; then \
		echo "[run-example] example IR not found: $(EXAMPLE_IR)" >&2; \
		echo "[run-example] add cue/examples/ssh_bruteforce.cue in Phase 1, then run: make cue-export" >&2; \
		exit 1; \
	fi
	@mkdir -p "$(dir $(RUN_OUT))"
	$(CARGO) run -- generate --ir "$(EXAMPLE_IR)" --out "$(RUN_OUT)" --seed "$(SEED)"

clean-generated: ## Remove generated example IR files.
	@find generated/examples -type f -name '*.json' -delete 2>/dev/null || true
