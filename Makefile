# Elfiee Project Makefile

.PHONY: test clippy clean fmt serve help

.DEFAULT_GOAL := help

test:
	cargo test

clippy:
	cargo clippy

clean:
	rm -rf target/

fmt:
	cargo fmt --all

serve:
	cargo run --bin elf -- serve --port 47200

help:
	@echo "Elfiee Makefile:"
	@echo ""
	@echo "  test   - cargo test"
	@echo "  clippy - cargo clippy"
	@echo "  clean  - rm -rf target/"
	@echo "  fmt    - cargo fmt --all"
	@echo "  serve  - elf serve --port 47200"
	@echo "  help   - show this help"
