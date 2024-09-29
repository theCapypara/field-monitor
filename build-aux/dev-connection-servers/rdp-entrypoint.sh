#!/usr/bin/env bash

# From https://github.com/scottyhardy/docker-remote-desktop/blob/master/entrypoint.sh
# License: MIT

# Create the user account
groupadd --gid 1020 ubuntu
useradd --shell /bin/bash --uid 1020 --gid 1020 --password $(openssl passwd ubuntu) --create-home --home-dir /home/ubuntu ubuntu
usermod -aG sudo ubuntu
chpasswd <<< "ubuntu:ubuntu"

# Start xrdp sesman service
/usr/sbin/xrdp-sesman

# Run xrdp in foreground if no commands specified
if [ -z "$1" ]; then
    /usr/sbin/xrdp --nodaemon
else
    /usr/sbin/xrdp
    exec "$@"
fi
