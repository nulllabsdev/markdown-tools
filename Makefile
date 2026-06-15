# Builds the markdown-tools CLIs for both language implementations.
#
#   make            # build all binaries into bin/
#   make go-align   # build only the Go aligner
#   make rust-align # build only the Rust aligner
#   make go-wrap    # build only the Go prose wrapper
#   make rust-wrap  # build only the Rust prose wrapper
#   make clean      # remove built binaries

BIN_DIR := bin

.PHONY: all go-align rust-align go-wrap rust-wrap clean

all: go-align rust-align go-wrap rust-wrap

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
