SHELL := /bin/sh
.DEFAULT_GOAL := help

UPSTREAM_URL ?= https://github.com/prankstr/vibepanel.git
UPSTREAM_BRANCH ?= main
LOCAL_BRANCH ?= main

.PHONY: help debug test build install update

help:
	@printf '%s\n' \
		'Usage: make <target>' \
		'' \
		'Targets:' \
		'  help     Show this help' \
		'  debug    Run the panel through run-debug.sh' \
		'  test     Run formatting, lint, unit, integration, UI, and font checks' \
		'  build    Run all tests and build the release workspace' \
		'  install  Test, package the committed HEAD with makepkg, and install it' \
		'  update   Merge upstream/main into a clean local main branch'

debug:
	./run-debug.sh

test:
	cargo fmt --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	./scripts/run-ui-regression-tests.sh
	./scripts/subset-font.sh --check

build: test
	cargo build --release --workspace

install:
	@if [ -n "$$(git status --porcelain)" ]; then \
		printf '%s\n' 'error: make install requires a clean worktree; commit the package contents first' >&2; \
		exit 1; \
	fi
	+@$(MAKE) test
	@set -eu; \
	head="$$(git rev-parse HEAD)"; \
	build_dir="$$(mktemp -d -t vibepanel-makepkg.XXXXXXXX)"; \
	cleanup() { rm -rf -- "$$build_dir"; }; \
	trap cleanup EXIT; \
	trap 'exit 129' HUP; \
	trap 'exit 130' INT; \
	trap 'exit 143' TERM; \
	cp -- PKGBUILD "$$build_dir/PKGBUILD"; \
	cd "$$build_dir"; \
	VIBEPANEL_REPO_URL="file://$(CURDIR)#commit=$$head" \
		makepkg --cleanbuild --clean --force --install \
		"BUILDDIR=$$build_dir/build" \
		"SRCDEST=$$build_dir/sources" \
		"PKGDEST=$$build_dir/packages" \
		"SRCPKGDEST=$$build_dir/srcpackages"

update:
	@set -eu; \
	branch="$$(git branch --show-current)"; \
	if [ "$$branch" != "$(LOCAL_BRANCH)" ]; then \
		printf 'error: make update must run on %s (current: %s)\n' "$(LOCAL_BRANCH)" "$$branch" >&2; \
		exit 1; \
	fi; \
	if [ -n "$$(git status --porcelain)" ]; then \
		printf '%s\n' 'error: make update requires a clean worktree' >&2; \
		exit 1; \
	fi; \
	if ! git remote get-url upstream >/dev/null 2>&1; then \
		git remote add upstream "$(UPSTREAM_URL)"; \
	elif [ "$$(git remote get-url upstream)" != "$(UPSTREAM_URL)" ]; then \
		printf 'error: upstream remote URL is %s (expected: %s)\n' \
			"$$(git remote get-url upstream)" "$(UPSTREAM_URL)" >&2; \
		exit 1; \
	fi; \
	git fetch upstream --prune --tags; \
	if ! git merge --no-edit "upstream/$(UPSTREAM_BRANCH)"; then \
		git merge --abort >/dev/null 2>&1 || true; \
		printf '%s\n' 'error: upstream merge failed and was aborted' >&2; \
		exit 1; \
	fi
