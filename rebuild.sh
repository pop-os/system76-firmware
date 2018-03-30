#!/usr/bin/env bash

set -ex

sudo systemctl stop system76-firmware-daemon || true
sudo make uninstall
make
sudo make install
sudo systemctl daemon-reload
sudo systemctl reload dbus
sudo systemctl start system76-firmware-daemon
