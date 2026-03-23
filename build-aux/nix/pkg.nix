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
  gcr_4,
  gtk-vnc,
  usbredir,
  libepoxy,
  libGL,
  openssl,
}:
stdenv.mkDerivation rec {
  pname = "field-monitor";
  version = "50.0";

  strictDeps = true;

  src = "${../..}";

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = "${src}/Cargo.lock";
    outputHashes = {
      "cbindgen-0.28.0" = "sha256-+uugMyqc55uj6EGBWQ9eCAA1xGlJGErlX9fvsbGk8Mg=";
      "gvnc-0.7.0" = "sha256-PGPaFJn0BXldnDbyDPaWU+hnLRhmTt3E6y+7mcuqyzk=";
      "spice-client-glib-0.7.0" = "sha256-SS0pYng9PSHGjtoxpiILeLAT6pLBWdRBzvSQwbzxmfU=";
    };
  };

  mesonBuildType = "release";

  nativeBuildInputs = [
    appstream
    appstream-glib
    blueprint-compiler
    cargo
    desktop-file-utils
    libxml2
    meson
    ninja
    pkg-config
    rustc
    rustPlatform.cargoSetupHook
    wrapGAppsHook4
  ];

  buildInputs = [
    glib
    gsettings-desktop-schemas
    gtk4
    gtk-vnc
    vte-gtk4
    gcr_4
    libadwaita
    libepoxy
    libGL
    libvirt
    openssl
    spice-gtk
    spice-protocol
    usbredir
    xdg-desktop-portal
  ]
  ++ (with gst_all_1; [
    gstreamer
    gst-plugins-base
    gst-plugins-good
  ]);

  postInstall = ''
    wrapProgram $out/bin/de.capypara.FieldMonitor --prefix PATH ':' "$out/libexec" --set RUST_LOG 'field_monitor=info,libfieldmonitor=info,GLib=info,warning'
  '';

  doCheck = true;

  meta = with lib; {
    description = "Viewer for virtual machines and other external screens";
    homepage = "https://github.com/theCapypara/field-monitor";
    license = licenses.gpl3Plus;
    mainProgram = "de.capypara.FieldMonitor";
  };
}
