PROJECT_NAME := systemd-tui
REGISTRY ?= ghcr.io/ptrvsrg
IMAGE_NAME ?= $(REGISTRY)/$(PROJECT_NAME)
VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo dev)
DOCKER_PLATFORM ?= linux/amd64
COMPOSE_SANDBOX_FILE ?= compose.sandbox.yaml

##@ General

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } /^[^[:space:]]+:.*##/ { gsub(/^[ \t]+|[ \t]+$$/, "", $$2); printf "  \033[36m%-28s\033[0m %s\n", $$1, $$2 } ' $(MAKEFILE_LIST)

##@ Verification

.PHONY: verify/fmt
verify/fmt: ## Check Rust formatting (cargo fmt --check).
	cargo fmt --check

.PHONY: verify/check
verify/check: ## Type-check project (cargo check).
	cargo check

.PHONY: verify/clippy
verify/clippy: ## Run clippy with warnings denied.
	cargo clippy --all-targets --all-features -- -D warnings

.PHONY: verify/all
verify/all: verify/fmt verify/check verify/clippy ## Run all static verification targets.

##@ Testing

.PHONY: test/unit
test/unit: ## Run unit tests.
	cargo test

.PHONY: test/all
test/all: test/unit ## Run all test targets.

##@ Build

.PHONY: build/debug
build/debug: ## Build debug binary.
	cargo build

.PHONY: build/release
build/release: ## Build release binary.
	cargo build --release

.PHONY: build/docker
build/docker: ## Build runtime Docker image.
	docker build \
		--platform $(DOCKER_PLATFORM) \
		-t $(IMAGE_NAME):$(VERSION) \
		-t $(IMAGE_NAME):latest \
		-f Dockerfile \
		.

.PHONY: build/docker-sandbox
build/docker-sandbox: ## Build sandbox systemd Docker image.
	docker build \
		--platform $(DOCKER_PLATFORM) \
		-t $(PROJECT_NAME)-sandbox:$(VERSION) \
		-f Dockerfile.sandbox \
		.

##@ Run

.PHONY: run/local
run/local: ## Run app locally, pass args with ARGS="...".
	cargo run -- $(ARGS)

.PHONY: run/sandbox-up
run/sandbox-up: ## Start sandbox container in background.
	docker compose -f $(COMPOSE_SANDBOX_FILE) up --build -d

.PHONY: run/sandbox-down
run/sandbox-down: ## Stop and remove sandbox container.
	docker compose -f $(COMPOSE_SANDBOX_FILE) down

.PHONY: run/sandbox-shell
run/sandbox-shell: ## Open shell inside sandbox container.
	docker exec -it systemd-tui-sandbox bash

##@ Release

.PHONY: release/deps-zigbuild
release/deps-zigbuild: ## Ensure zig and cargo-zigbuild are installed.
	@command -v zig >/dev/null 2>&1 || { \
		echo "zig is required for cross-linking (cargo zigbuild)."; \
		echo "Install zig first (e.g. brew install zig)."; \
		exit 1; \
	}
	@cargo zigbuild --help >/dev/null 2>&1 || cargo install cargo-zigbuild --locked

.PHONY: release/deps-snapcraft
release/deps-snapcraft: ## Ensure snapcraft is installed.
	@command -v snapcraft >/dev/null 2>&1 || { \
		echo "snapcraft is required by GoReleaser (snapcrafts section)."; \
		echo "Install it manually (e.g. brew install snapcraft or sudo snap install snapcraft --classic)."; \
		exit 1; \
	}

.PHONY: release/deps
release/deps: release/deps-zigbuild release/deps-snapcraft ## Ensure release dependencies are installed (zig + cargo-zigbuild + snapcraft).

.PHONY: release/check
release/check: ## Validate GoReleaser configuration.
	goreleaser check

.PHONY: release/snapshot
release/snapshot: release/deps ## Build local GoReleaser snapshot (no publish).
	goreleaser release --snapshot --clean --skip=publish

##@ Cleaning

.PHONY: clean/all
clean/all: ## Remove Rust build artifacts.
	cargo clean

.DEFAULT_GOAL := help
