prefix ?= /usr
sysconfdir ?= /etc
exec_prefix = $(prefix)
bindir = $(exec_prefix)/bin
libdir = $(exec_prefix)/lib
includedir = $(prefix)/include
datarootdir = $(prefix)/share
datadir = $(datarootdir)

CARGO_BIN ?= cargo
SRC = Cargo.toml Cargo.lock Makefile $(shell find src -type f -wholename '*src/*.rs')

.PHONY: all clean distclean install uninstall update

PKG=system76-firmware
CLI=$(PKG)-cli
DAEMON=$(PKG)-daemon

ARGS = --release
VENDORED ?= 0
ifeq ($(VENDORED),1)
	ARGS += --frozen
endif

all: target/release/$(CLI) target/release/$(DAEMON)

clean:
	$(CARGO_BIN) clean

distclean: clean
	rm -rf .cargo vendor vendor.tar.xz

install: install-cli install-daemon

install-cli: target/release/$(CLI)
	install -D -m 0755 "target/release/$(CLI)" "$(DESTDIR)$(bindir)/$(CLI)"

install-daemon: target/release/$(DAEMON)
	install -D -m 0755 "target/release/$(DAEMON)" "$(DESTDIR)$(libdir)/$(PKG)/$(DAEMON)"
	install -D -m 0644 "data/$(DAEMON).conf" "$(DESTDIR)$(sysconfdir)/dbus-1/system.d/$(DAEMON).conf"
	install -D -m 0644 "debian/$(DAEMON).service" "$(DESTDIR)$(sysconfdir)/systemd/system/$(DAEMON).service"

uninstall: uninstall-cli uninstall-daemon

uninstall-cli:
	rm -f "$(DESTDIR)$(bindir)/$(CLI)"

uninstall-daemon:
	rm -f "$(DESTDIR)$(libdir)/$(PKG)/$(DAEMON)"
	rm -f "$(DESTDIR)$(sysconfdir)/dbus-1/system.d/$(DAEMON).conf"
	rm -f "$(DESTDIR)$(sysconfdir)/systemd/system/$(DAEMON).service"

update:
	cargo update

vendor:
	mkdir -p .cargo
	cargo vendor --sync daemon/Cargo.toml | head -n -1 > .cargo/config.toml
	echo 'directory = "vendor"' >> .cargo/config.toml
	tar pcfJ vendor.tar.xz vendor
	rm -rf vendor

target/release/$(CLI) target/release/$(DAEMON): $(SRC)
ifeq ($(VENDORED),1)
	tar pxf vendor.tar.xz
endif
	$(CARGO_BIN) build $(ARGS)
	$(CARGO_BIN) build -p $(DAEMON) $(ARGS)
