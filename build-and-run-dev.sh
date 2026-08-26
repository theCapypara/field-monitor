#!/usr/bin/env bash
set -xe
# You can use this script to build the Devel app using just Flatpak.
# `org.Flatpak.Builder` flatpak must be installed. SDKs, Platforms and extensions must be installed.

cd build-aux/flatpak

flatpak-builder \
  --user \
  --force-clean \
  --repo=../../build/flatpak/fm-repo \
  --state-dir ../../build/flatpak/fm-state-dir \
  ../../build/flatpak/fm-build-dir \
  de.capypara.FieldMonitor.Devel.json

cd ../..

flatpak --user install --reinstall --noninteractive --include-debug ./build/flatpak/fm-repo/ \
  de.capypara.FieldMonitor.Devel

exec flatpak run de.capypara.FieldMonitor.Devel
