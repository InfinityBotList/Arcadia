all:
	cargo build --release
restartwebserver:
	make all
	make restartwebserver_nobuild

restartwebserver_nobuild:
	sudo systemctl stop arcadia
	sleep 3 # Give time for it to stop
	cp -v target/release/arcadia arcadia
	sudo systemctl start arcadia

