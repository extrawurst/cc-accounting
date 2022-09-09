run:
	RUST_LOG=debug cargo r -- ./cc-2022-06/table.csv

check:
	cargo clippy
	cargo fmt --check
