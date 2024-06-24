format:
	cargo fmt --all
	cargo clippy --all-features

debug: 
	maturin develop

release:
	maturin develop --release