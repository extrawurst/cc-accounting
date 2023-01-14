run:
	RUST_LOG=debug cargo r -- ./cc-2022-06/table.csv

check:
	cargo make checks

bundle:
	cargo make bundle
