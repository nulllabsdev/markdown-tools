# Builds the markdown-tools CLIs for both language implementations.
#
#   make            # build all binaries into bin/
#   make go-align   # build only the Go aligner
#   make rust-align # build only the Rust aligner
#   make go-wrap    # build only the Go prose wrapper
#   make rust-wrap  # build only the Rust prose wrapper
#   make test       # run all tests (both tools, both languages)
#   make clean      # remove built binaries

BIN_DIR := bin

.PHONY: all go-align rust-align go-wrap rust-wrap test test-go test-rust clean

all: go-align rust-align go-wrap rust-wrap

# Tests are per language: `go test ./...` covers the aligner and wrapper Go
# packages and commands; `cargo test` covers both Rust binaries and the lib.
test: test-go test-rust

test-go:
	cd go && go test ./...

test-rust:
	cd rust && cargo test

go-align:
	mkdir -p $(BIN_DIR)
	cd go && go build -o ../$(BIN_DIR)/go-align ./cmd/go-align

go-wrap:
	mkdir -p $(BIN_DIR)
	cd go && go build -o ../$(BIN_DIR)/go-wrap ./cmd/go-wrap

rust-align:
	cd rust && cargo build --release --bin rust-align
	mkdir -p $(BIN_DIR)
	cp rust/target/release/rust-align $(BIN_DIR)/rust-align

rust-wrap:
	cd rust && cargo build --release --bin rust-wrap
	mkdir -p $(BIN_DIR)
	cp rust/target/release/rust-wrap $(BIN_DIR)/rust-wrap

clean:
	rm -rf $(BIN_DIR)
	cd rust && cargo clean
