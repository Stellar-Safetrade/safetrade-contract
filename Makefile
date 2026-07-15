CONTRACT_WASM := target/wasm32-unknown-unknown/release/safetrade_escrow.wasm
SOURCE_ACCOUNT := YOUR_ACCOUNT
NETWORK := testnet

.PHONY: build test deploy-testnet fmt lint clean

build:
	cargo build --target wasm32-unknown-unknown --release

test:
	cargo test

deploy-testnet: build
	stellar contract deploy \
	  --wasm $(CONTRACT_WASM) \
	  --source $(SOURCE_ACCOUNT) \
	  --network $(NETWORK)

fmt:
	cargo fmt

lint:
	cargo clippy --all-targets -- -D warnings

clean:
	cargo clean
