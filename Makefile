CARGO_FLAGS := -Zbuild-std=core,compiler_builtins,alloc \
               -Zbuild-std-features=compiler-builtins-mem
CARGO_BUILD := cargo +nightly build $(CARGO_FLAGS)

x86_64-debug:
	$(CARGO_BUILD) --target targets/x86_64-nucloid.json

x86_64-release:
	$(CARGO_BUILD) --release --target targets/x86_64-nucloid.json

tests:
	cargo +nightly test

.PHONY: x86_64-debug x86_64-release tests
