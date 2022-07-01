include .env

RUSTFLAGS_LOCAL="-C target-cpu=native $(RUSTFLAGS) -C link-arg=-fuse-ld=lld"
CARGO_TARGET_GNU_LINKER="x86_64-unknown-linux-gnu-gcc"

# Some sensible defaults, should be overrided per-project
BIN_NAME ?= bot
PROJ_NAME ?= arcadia
HOST ?= 100.86.85.125

all: 
	@make cross
dev:
	DATABASE_URL=$(DATABASE_URL) RUSTFLAGS=$(RUSTFLAGS_LOCAL) cargo build
devrun:
	DATABASE_URL=$(DATABASE_URL) RUSTFLAGS=$(RUSTFLAGS_LOCAL) cargo run
run:
	-mv -vf $(BIN_NAME).new $(BIN_NAME) # If it exists
	./$(BIN_NAME)
cross:
	DATABASE_URL=$(DATABASE_URL) CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=$(CARGO_TARGET_GNU_LINKER) cargo build --target=x86_64-unknown-linux-gnu --release
push:
	scp -C target/x86_64-unknown-linux-gnu/release/$(BIN_NAME) root@$(HOST):$(PROJ_NAME)/$(BIN_NAME).new
remote:
	ssh root@$(HOST)
up:
	git submodule foreach git pull

