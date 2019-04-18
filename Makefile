prefix ?= /usr
sysconfdir ?= /etc
exec_prefix = $(prefix)
bindir = $(exec_prefix)/bin
libdir = $(exec_prefix)/lib
includedir = $(prefix)/include
datarootdir = $(prefix)/share
datadir = $(datarootdir)

.PHONY: all clean distclean install uninstall update

PKG=system76-firmware
CLI=$(PKG)-cli
DAEMON=$(PKG)-daemon

all: target/release/$(CLI) target/release/$(DAEMON)

clean:
	cargo clean

distclean: clean
	rm -rf .cargo vendor

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

target/release/$(CLI) target/release/$(DAEMON): Cargo.lock Cargo.toml src/* src/*/*
	if [ -d vendor ]; \
	then \
		cargo build --release --frozen; \
	else \
		cargo build --release; \
	fi
