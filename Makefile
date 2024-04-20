CDN_PATH := /silverpelt/cdn/ibl

all:
	cargo build --release
restartwebserver:
	cargo sqlx prepare
	make all
	make restartwebserver_nobuild

restartwebserver_nobuild:
	sudo systemctl stop arcadia
	sleep 3 # Give time for it to stop
	cp -v target/release/bot bot
	sudo systemctl start arcadia

ts:
	rm -rvf $(CDN_PATH)/dev/bindings/arcadia
	cargo test
	cp -rf bindings/.generated $(CDN_PATH)/dev/bindings/arcadia
