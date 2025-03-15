#!/bin/sh
set -xe

meson setup build
exec meson compile "$1" -C build
