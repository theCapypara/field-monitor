## Code of Conduct

Field Monitor follows the [GNOME Code of Conduct](https://conduct.gnome.org/).

- **Be friendly.** Use welcoming and inclusive language.
- **Be empathetic.** Be respectful of differing viewpoints and experiences.
- **Be respectful.** When we disagree, we do so in a polite and constructive manner.
- **Be considerate.** Remember that decisions are often a difficult choice between competing priorities.
- **Be patient and generous.** If someone asks for help it is because they need it.
- **Try to be concise.** Read the discussion before commenting.

## Repository Structure

### Source Code:

- [/src](src): Crate for the main GTK App
- [/lib](lib): Library crate, used by all other crates
- [/connection](connection): Sub-crates that all implement one (or more) type(s) of connection(s)
- [/vte-pty-driver](vte-pty-driver):
  Crates that compile into small binaries to drive the console for some connections, like the Proxmox
  or libvirt console connections. These are connected/started by VTE as a subprocess via PTY. The
  [lib](vte-pty-driver/lib) subdirectory contains shared library code between these libraries.

### Data & Localization:

- [/data](data): Metainfo, desktop file, icons, gschema, etc.
- [/po](po): Localization files

### CI, Building & Tooling:

- [/.github](.github): GitHub Actions CI system
- [/build-aux](build-aux)
    - [/dev-connection-servers](build-aux/dev-connection-servers):
      Useful Docker images to quickly start VNC, RDP and SPICE servers to connect to during
      development.
    - [/flatpak](build-aux/flatpak): Flatpak definitions
    - [/gettext](build-aux/gettext): Utility configuration for gettext
    - [/nix](build-aux/nix): Nix definitions
- [/subprojects](subprojects): Meson subprojects to use, currently contains Blueprint Compiler

## Building Field Monitor

In order to build Field Monitor the [Meson](https://mesonbuild.com/) build system must be used.

You will need the following dependencies to successfully build and run Field Monitor:

- Meson
- Components included in the current stable or nightly
  [GNOME SDK](https://developer.gnome.org/documentation/introduction/components.html),
  see the [Flatpak manifests](build-aux/flatpak/de.capypara.FieldMonitor.Devel.json)
  for reference on which version should be used.
    - Not all components are used, but a lot are, most importantly GTK 4 and libadwaita and all their dependencies
- Current Rust stable release, including Cargo
- Dependencies of [RDW](https://gitlab.gnome.org/malureau/rdw):
    - libusb
    - usbredir
    - gtk-vnc 1.5+
    - spice-gtk
    - freerdp
- VTE4
- libvirt

### Using GNOME Builder / Flatpak

You can also develop Field Monitor by using [GNOME Builder](https://apps.gnome.org/en/Builder/) and the development
Flatpak manifest
at [build-aux/flatpak/de.capypara.FieldMonitor.Devel.json](build-aux/flatpak/de.capypara.FieldMonitor.Devel.json).

## Localization

Field Monitor can currently only be localized locally, there is no web-based platform yet.

To localize Field Monitor, generate an up-to-date pot file using:

```sh
# Once:
meson setup build
# To rebuild the pot file:
meson compile de.capypara.FieldMonitor-pot -C build
```

You can then use this pot file to update or generate the po files per locale. Update the `data/LINGUAS` if
you add a new language.
