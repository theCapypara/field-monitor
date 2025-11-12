{
  enableDebugging,
  stdenv,
  field-monitor,
  lib,
  glib,
  gsettings-desktop-schemas,
  gtk4,
  gtk-vnc,
  vte-gtk4,
  libadwaita,
  libepoxy,
  libGL,
  libvirt,
  openssl,
  spice-gtk,
  spice-protocol,
  usbredir,
  xdg-desktop-portal,
  gst_all_1
}:
stdenv.mkDerivation {
  pname = "field-monitor-devel";

  strictDeps = true;
  dontStrip = true;
  enableDebugging = true;

  inherit (field-monitor)
    src
    cargoDeps
    nativeBuildInputs
    version
    ;

  # To actually get all debug symbols for some libs, nixseparatedebuginfod2 is needed!
  buildInputs = [
    (enableDebugging glib)
    gsettings-desktop-schemas
    (enableDebugging gtk4)
    (enableDebugging gtk-vnc)
    (enableDebugging vte-gtk4)
    (enableDebugging libadwaita)
    libepoxy
    libGL
    libvirt
    openssl
    (enableDebugging spice-gtk)
    (enableDebugging spice-protocol)
    usbredir
    xdg-desktop-portal
  ]
  ++ (with gst_all_1; [
    gstreamer
    gst-plugins-base
    gst-plugins-good
  ]);

  mesonBuildType = "debug";
  mesonFlags = [
    "-Dapp-id=de.capypara.FieldMonitor.Devel"
  ];

  postInstall = ''
    wrapProgram $out/bin/de.capypara.FieldMonitor.Devel --prefix PATH ':' "$out/libexec" --set RUST_LOG 'field_monitor=trace,libfieldmonitor=trace,oo7=debug,rdw=trace,rdw-vnc=trace,rdw-spice=trace,rdw-rdp=trace,vte=debug,gtk-vnc=debug,Adwaita=info,GLib=info,warning' --set RUST_BACKTRACE '1'
  '';

  doCheck = true;

  meta = with lib; {
    description = "Viewer for virtual machines and other external screens";
    homepage = "https://github.com/theCapypara/field-monitor";
    license = licenses.gpl3Plus;
    mainProgram = "de.capypara.FieldMonitor.Devel";
  };
}
