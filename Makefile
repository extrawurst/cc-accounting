run:
	RUST_LOG=debug cargo r

check:
	cargo make checks

bundle:
	cargo make bundle
	cp -r target/debug/bundle/osx/ccaccounting.app . 
