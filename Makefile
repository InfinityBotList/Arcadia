CDN_PATH := /silverpelt/cdn/ibl

# Read current env from current-env file
CURRENT_ENV := $(shell cat current-env)

all:
	cargo build --release
restartwebserver:
	cargo sqlx prepare
	make all
	make restartwebserver_nobuild

restartwebserver_nobuild:
	systemctl stop arcadia-${CURRENT_ENV}
	sleep 3 # Give time for it to stop
	cp -v target/release/bot bot
	systemctl start arcadia-${CURRENT_ENV}

promoteprod:
	rm -rf ../prod2
	cd .. && cp -rf staging prod2
	echo "prod" > ../prod2/current-env
	cd ../prod2 && make restartwebserver && rm -rf ../prod && mv -vf ../prod2 ../prod && systemctl restart arcadia-prod
	cd ../prod && make ts
	# Git push to "current-prod" branch
	cd ../prod && git branch current-prod && git add -v . && git commit -m "Promote staging to prod" && git push -u origin HEAD:current-prod --force

ts:
	rm -rvf $(CDN_PATH)/dev/bindings/arcadia
	cargo test
	cp -rf bindings/.generated $(CDN_PATH)/dev/bindings/arcadia
