#!/usr/bin/env bash

set -ex

cargo build
sudo cp data/com.system76.firmwaredaemon.conf /etc/dbus-1/system.d/com.system76.firmwaredaemon.conf
sudo systemctl reload dbus
sudo target/debug/system76-firmware-daemon
