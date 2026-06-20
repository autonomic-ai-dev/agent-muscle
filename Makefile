.PHONY: all build test check clean release sign install

all: build

build:
	cargo build --release -p agent-muscle

test:
	cargo test --release -p agent-muscle

check:
	cargo check -p agent-muscle

clean:
	cargo clean

release: build
	@echo "Release build complete"

sign:
	@scripts/sign-macos.sh

install:
	@scripts/sync-local-release.sh
