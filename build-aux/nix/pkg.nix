{
  stdenv,
  lib,
  fetchgit,
  cargo,
  meson,
  ninja,
  rustPlatform,
  rustc,
  pkg-config,
  glib,
  gsettings-desktop-schemas,
  gtk4,
  libadwaita,
  libvirt,
  gst_all_1,
  desktop-file-utils,
  appstream,
  appstream-glib,
  wrapGAppsHook4,
  xdg-desktop-portal,
  blueprint-compiler,
  libxml2,
  freerdp,
  spice-protocol,
  spice-gtk,
  vte-gtk4,
  gtk-vnc,
  usbredir,
  libepoxy,
  libGL,
  openssl,
}:
let
  patched-gtk-vnc = (
    gtk-vnc.dev.overrideAttrs (
      finalAttrs: previousAttrs: {
        version = "1.3.1+ca";
        src = fetchgit {
          # see note in flatpak sources
          url = "https://gitlab.gnome.org/theCapypara/gtk-vnc.git";
          rev = "ad14f260652e07aa2e7fc7481b7d998855160d2d";
          hash = "sha256-5jXy0YMDrBSwlqMUS9NGDz5QLe2bi1LOUhLAQlTHhCI=";
        };
      }
    )
  );
in
stdenv.mkDerivation rec {
  pname = "field-monitor";
  version = "47.0";

  src = "${../..}";

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = "${src}/Cargo.lock";
    outputHashes = {
      "cbindgen-0.20.0" = "sha256-ijm2ExZUHG62MU0mr3FXoC35vyRbC33kKEKr7ysk6hQ=";
      "freerdp2-0.2.0" = "sha256-e1kb4vFCUs+dKHhSVCt5DMoFqc3fjtgChv+Z/g0ItUE=";
    };
  };

  prodNativeBuildInputs = [
    glib
    gtk4
    meson
    ninja
    pkg-config
    rustPlatform.bindgenHook
    rustPlatform.cargoSetupHook
    cargo
    rustc
    desktop-file-utils
    appstream
    appstream-glib
    wrapGAppsHook4
    blueprint-compiler
    libxml2
    freerdp
    spice-protocol
    spice-gtk
    usbredir
    libepoxy
    libGL
    openssl
  ];

  nativeBuildInputs = prodNativeBuildInputs ++ [
    vte-gtk4
    patched-gtk-vnc
  ];

  mesonBuildType = "release";

  postInstall = ''
    wrapProgram $out/bin/de.capypara.FieldMonitor --prefix PATH ':' "$out/bin" --set RUST_LOG 'field_monitor=info,libfieldmonitor=info,GLib=info,warning'
  '';

  doCheck = true;

  buildInputs =
    [
      glib
      gtk4
      gsettings-desktop-schemas
      libadwaita
      libvirt
      xdg-desktop-portal
      openssl
    ]
    ++ (with gst_all_1; [
      gstreamer
      gst-plugins-base
      gst-plugins-good
    ]);

  meta = with lib; {
    description = "XXXXXXXXXXXXXXX";
    homepage = "https://github.com/theCapypara/field-monitor";
    license = licenses.gpl3Plus;
    mainProgram = "de.capypara.FieldMonitor";
  };
}
