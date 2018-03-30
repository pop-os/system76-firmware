# system76-firmware-daemon

The system76-firmware-daemon package has a systemd service which exposes a DBUS API for handling firmware updates.

## Dependencies

- cargo
- dbus
- rustc
- systemd

## Make Targets

The following make targets are supported:

- `make all` - compile all binaries
- `make clean` - remove compiled binaries
- `make install` - install binaries and configuration files
- `make uninstall` - uninstall binaries and configuration file
- `make vendor` - prepare source for offline compilation
- `make distclean` - remove prepared source and compiled binaries

## Installation

```
make
sudo make install
```

## Packaging

In order to package this, you need `cargo-vendor`:

```
cargo install cargo-vendor
```

You can then run the following to create an offline-capable package:

```
make vendor
```

Now you can compile and install the package.

To clean out the vendor source, you can run this command:

```
make distclean
```
