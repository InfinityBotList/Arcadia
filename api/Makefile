include ../.env

RUSTFLAGS_LOCAL="-C target-cpu=native $(RUSTFLAGS) -C link-arg=-fuse-ld=lld"
CARGO_TARGET_GNU_LINKER="x86_64-unknown-linux-gnu-gcc"

# Some sensible defaults, should be overrided per-project
BIN_NAME ?= api
PROJ_NAME ?= arcadia/api
HOST ?= 100.86.85.125

run:
	-mv -vf $(BIN_NAME).new $(BIN_NAME) # If it exists
	./$(BIN_NAME)
