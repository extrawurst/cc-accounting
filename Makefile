run:
	RUST_LOG=debug cargo r 

check:
	cargo clippy
	cargo fmt --check
