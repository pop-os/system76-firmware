# system76-firmware

The system76-firmware package has a CLI tool for installing firmware updates.

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

## API

The system76-firmware-daemon will download the latest firmware package, if it has
changed, and will provide a DBUS interface for a user to query the current firmware
status, query the update information, and schedule an update.

The DBUS API is as follows:

- `Bios() -> (String model, String version)`
  Query the BIOS model and version.
- `EmbeddedController(Boolean primary) -> (String project, String version)`
  Query the embedded controller for project and version. Optionally, a second
  embedded controller can be queried.
- `ManagementEngine() -> (Boolean enabled, String version)`
  Query the ME status and version.
- `Download() -> (String digest, String changelog)`
  Download the latest changelog information
- `Schedule(String digest) -> ()`
  Prepare the latest firmware update for installation
- `Unschedule() -> ()`
  Cancel installation of the latest firmware update
