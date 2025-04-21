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
  version = "40.1";

  src = "${../..}";

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = "${src}/Cargo.lock";
    outputHashes = {
      "cbindgen-0.28.0" = "sha256-hSNu91if//1e5YFDdr5wzACeKkGMxx042C5L8adalWg=";
      "ironrdp-0.9.1" = "sha256-TK2/Pjnui12yWaHYD5VGooJ0fC6YAY8W4n3ZrJVaQMs=";
    };
  };

  mesonBuildType = "release";

  postInstall = ''
    wrapProgram $out/bin/de.capypara.FieldMonitor --prefix PATH ':' "$out/libexec" --set RUST_LOG 'field_monitor=info,libfieldmonitor=info,GLib=info,warning'
  '';

  doCheck = true;

  prodNativeBuildInputs = [
    glib
    gtk4
    meson
    ninja
    pkg-config
    rustPlatform.cargoSetupHook
    cargo
    rustc
    desktop-file-utils
    appstream
    appstream-glib
    wrapGAppsHook4
    blueprint-compiler
    libxml2
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
