APP := dota_2_gsi_invoker
WINDOWS_TARGET := x86_64-pc-windows-gnu
WINDOWS_DIST := release/windows

.PHONY: init run test release-windows

init:
	rustup component add rustfmt
	rustup component add clippy
	rustup target add $(WINDOWS_TARGET)
	cargo fetch

run:
	cargo run

test:
	cargo fmt --check
	cargo test
	cargo clippy --all-targets -- -D warnings

release-windows:
	rustup target add $(WINDOWS_TARGET)
	cargo build --release --target $(WINDOWS_TARGET)
	rm -rf $(WINDOWS_DIST)
	mkdir -p $(WINDOWS_DIST)
	cp target/$(WINDOWS_TARGET)/release/$(APP).exe $(WINDOWS_DIST)/
	cp dota_2_gsi_invoker_config.json $(WINDOWS_DIST)/
	cp gamestate_integration_dota_2_gsi_invoker.cfg $(WINDOWS_DIST)/
