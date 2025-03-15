FROM fedora:42

# Install Rust using Rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN yum install -y \
    meson gcc gcc-c++ gettext desktop-file-utils glib2-devel \
    gtk-update-icon-cache wayland-devel egl-wayland-devel \
    libepoxy-devel openssl-devel cairo-devel spice-glib-devel \
    freerdp2-devel cairo-gobject-devel gvnc-devel gtk4-devel \
    libadwaita-devel vte291-gtk4-devel \
    && yum clean all && rm -rf /var/cache/yum

COPY entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
