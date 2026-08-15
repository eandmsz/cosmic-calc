# Minimal install target. `cargo build --release` first, or just run
# `make` which does it for you.
PREFIX ?= /usr/local
DESTDIR ?=
APPID = io.github.eandmsz.CosmicCalc

BINDIR = $(DESTDIR)$(PREFIX)/bin
APPDIR = $(DESTDIR)$(PREFIX)/share/applications
METADIR = $(DESTDIR)$(PREFIX)/share/metainfo

.PHONY: all build install uninstall check clean

all: build

build:
	cargo build --release

# Fast loop: the core has no GUI dependencies and tests in seconds.
check:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace

install: build
	install -Dm755 target/release/cosmic-calc $(BINDIR)/cosmic-calc
	install -Dm644 res/$(APPID).desktop $(APPDIR)/$(APPID).desktop
	install -Dm644 res/$(APPID).metainfo.xml $(METADIR)/$(APPID).metainfo.xml

uninstall:
	rm -f $(BINDIR)/cosmic-calc
	rm -f $(APPDIR)/$(APPID).desktop
	rm -f $(METADIR)/$(APPID).metainfo.xml

clean:
	cargo clean
