default: build

all: test

test: build
	cargo test

build:
	stellar contract build --out-dir target/stellar
	stellar contract optimize --wasm target/stellar/soroban_admin_contract.wasm --wasm-out target/stellar/soroban_admin_contract.wasm
	@ls -l target/stellar/*.wasm

fmt:
	cargo fmt --all

clean:
	cargo clean
