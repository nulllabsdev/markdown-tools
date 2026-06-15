# Builds the markdown table-alignment CLIs for both language implementations.
#
#   make            # build both binaries into bin/
#   make go-align   # build only the Go binary
#   make rust-align # build only the Rust binary
#   make clean      # remove built binaries

BIN_DIR := bin

.PHONY: all go-align rust-align clean

all: go-align rust-align

go-align:
	mkdir -p $(BIN_DIR)
	cd go && go build -o ../$(BIN_DIR)/go-align ./cmd/go-align

rust-align:
	cd rust && cargo build --release --bin rust-align
	mkdir -p $(BIN_DIR)
	cp rust/target/release/rust-align $(BIN_DIR)/rust-align

clean:
	rm -rf $(BIN_DIR)
	cd rust && cargo clean
