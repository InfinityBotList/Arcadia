include .env

RUSTFLAGS_LOCAL="-C target-cpu=native $(RUSTFLAGS) -C link-arg=-fuse-ld=lld"
CARGO_TARGET_GNU_LINKER="x86_64-unknown-linux-gnu-gcc"

# Some sensible defaults, should be overrided per-project
BINS ?= bot
PROJ_NAME ?= arcadia
HOST ?= 100.71.175.17
BINDINGS_URL ?= /iblcdn/public/dev/apiBindings

all: 
	@make cross
dev:
	DATABASE_URL=$(DATABASE_URL) RUSTFLAGS=$(RUSTFLAGS_LOCAL) cargo build
devrun:
	DATABASE_URL=$(DATABASE_URL) RUSTFLAGS=$(RUSTFLAGS_LOCAL) cargo run
cross:
	DATABASE_URL=$(DATABASE_URL) CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=$(CARGO_TARGET_GNU_LINKER) cargo build --target=x86_64-unknown-linux-gnu --release ${ARGS}
push:
	# Kill arcadia
	ssh root@$(HOST) "systemctl stop arcadia"

	@for bin in $(BINS) ; do \
		echo "Pushing $$bin to $(HOST):${PROJ_NAME}/$$bin"; \
		scp -C target/x86_64-unknown-linux-gnu/release/$$bin root@$(HOST):${PROJ_NAME}/$$bin; \
	done

	# Start arcadia
	ssh root@$(HOST) "systemctl start arcadia"

	DATABASE_URL=$(DATABASE_URL) cargo test ${ARGS}

	ssh root@$(HOST) "mkdir -p ${BINDINGS_URL}"

	scp -r .generated root@${HOST}:${BINDINGS_URL}

	ssh root@$(HOST) "rm -rf ${BINDINGS_URL}/*.ts && cp ${BINDINGS_URL}/.generated/*.ts ${BINDINGS_URL}/ && rm -rf ${BINDINGS_URL}/.generated"

	@# Remove the .generated folder
	rm -rf .generated

remote:
	ssh root@$(HOST)
up:
	git submodule foreach git pull
run:
	-mv -vf bot.new bot # If it exists
	./bot
