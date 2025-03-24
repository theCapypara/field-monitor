#!/usr/bin/env bash
set -xe
# You can use this script to build the Devel app using just Flatpak.
# `org.Flatpak.Builder` flatpak must be installed. SDKs, Platforms and extensions must be installed.

cd build-aux/flatpak

flatpak-builder \
  --user \
  --force-clean \
  --repo=/tmp/fm-repo \
  --state-dir /tmp/fm-state-dir \
  /tmp/fm-build-dir \
  de.capypara.FieldMonitor.Devel.json
flatpak --user install --reinstall --noninteractive --include-debug /tmp/fm-repo/ \
  de.capypara.FieldMonitor.Devel

exec flatpak run de.capypara.FieldMonitor.Devel
