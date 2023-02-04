include .env

RUSTFLAGS_LOCAL="-C target-cpu=native $(RUSTFLAGS) -C link-arg=-fuse-ld=lld"
CARGO_TARGET_GNU_LINKER="x86_64-unknown-linux-gnu-gcc"

# Some sensible defaults, should be overrided per-project
BINS ?= api bot
PROJ_NAME ?= arcadia
HOST ?= 100.86.85.125

all: 
	@make cross
onlyapi:
	@make cross ARGS="--workspace --bin api"
onlybot:
	@make cross ARGS="--workspace --bin bot"
dev:
	DATABASE_URL=$(DATABASE_URL) RUSTFLAGS=$(RUSTFLAGS_LOCAL) cargo build
devrun:
	DATABASE_URL=$(DATABASE_URL) RUSTFLAGS=$(RUSTFLAGS_LOCAL) cargo run
cross:
	DATABASE_URL=$(DATABASE_URL) CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=$(CARGO_TARGET_GNU_LINKER) cargo build --target=x86_64-unknown-linux-gnu --release ${ARGS}
copybindings:
	DATABASE_URL=$(DATABASE_URL) cargo test ${ARGS}
	@# For every binding in the .generated folder created, copy via SCP to /iblseed/apiBindings

	ssh root@$(HOST) "mkdir -p /iblseeds/apiBindings"

	@for file in $(shell ls .generated) ; do \
		echo "Copying $$file to $(HOST):/iblseeds/apiBindings/$$(basename $$file)"; \
		scp -C .generated/$$file root@$(HOST):/iblseeds/apiBindings/$$(basename $$file); \
	done
push:
	@for bin in $(BINS) ; do \
		echo "Pushing $$bin to $(HOST):${PROJ_NAME}/$$bin/$$bin.new"; \
		scp -C target/x86_64-unknown-linux-gnu/release/$$bin root@$(HOST):${PROJ_NAME}/$$bin/$$bin.new; \
	done
	make copybindings
remote:
	ssh root@$(HOST)
up:
	git submodule foreach git pull
runapi:
	-mv -vf api/api.new api/api # If it exists
	./api/api
runbot:
	-mv -vf bot/bot.new bot/bot # If it exists
	./bot/bot