{
  stdenv,
  lib,
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
stdenv.mkDerivation rec {
  pname = "field-monitor";
  version = "48.0";

  src = "${../..}";

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = "${src}/Cargo.lock";
    outputHashes = {
      "cbindgen-0.20.0" = "sha256-6gBhf4MOgPvS/XokMmssEgXWQ9CTd848ms6H3DS3K70=";
      "freerdp2-0.2.0" = "sha256-LYDLaEzRGlEADTo1lRD8ArR1MrHhGHE4rRWjFURxggo=";
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
    gtk-vnc
  ];

  mesonBuildType = "release";

  postInstall = ''
    wrapProgram $out/bin/de.capypara.FieldMonitor --prefix PATH ':' "$out/libexec" --set RUST_LOG 'field_monitor=info,libfieldmonitor=info,GLib=info,warning'
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
    description = "Viewer for virtual machines and other external screens";
    homepage = "https://github.com/theCapypara/field-monitor";
    license = licenses.gpl3Plus;
    mainProgram = "de.capypara.FieldMonitor";
  };
}
