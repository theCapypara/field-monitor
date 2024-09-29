FROM alsadi/containerized-xorg-spice:latest@sha256:0f4896d560c1dbce17e8b96038d3ea6987ba44b7fb7d1a488568bd8963b129c0

USER root

RUN dnf install -y xrandr spice-vdagent

# A bit crazy how this isn't fixed: https://gitlab.freedesktop.org/xorg/driver/xf86-video-qxl/-/issues/5
RUN sed -i '/atexit.register(cleanup)/a temp_dir = None' /usr/bin/Xspice

USER app

ENTRYPOINT ["Xspice", "--vdagent", "--xsession"]
CMD ["openbox-session", ":1"]