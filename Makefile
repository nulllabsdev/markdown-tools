# Builds the markdown-tools CLIs for both language implementations.
#
#   make            # build both binaries into bin/
#   make gomd       # build the Go CLI
#   make rustmd     # build the Rust CLI
#   make test       # run all tests (both tools, both languages)
#   make clean      # remove built binaries

BIN_DIR := bin

.PHONY: all gomd rustmd test test-go test-rust clean

all: gomd rustmd

# Tests are per language: `go test ./...` covers the Go package and CLI;
# `cargo test` covers the Rust binary and the lib.
test: test-go test-rust

test-go:
	cd go && go test ./...

test-rust:
	cd rust && cargo test

gomd:
	mkdir -p $(BIN_DIR)
	cd go && go build -o ../$(BIN_DIR)/gomd ./cmd/gomd

rustmd:
	cd rust && cargo build --release --bin rustmd
	mkdir -p $(BIN_DIR)
	cp rust/target/release/rustmd $(BIN_DIR)/rustmd

clean:
	rm -rf $(BIN_DIR)
	cd rust && cargo clean
