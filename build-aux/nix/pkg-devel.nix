{
  field-monitor,
  stdenv,
  lib,
}:

stdenv.mkDerivation {
  pname = "field-monitor-devel";
  version = "47.0";

  inherit (field-monitor)
    src
    cargoDeps
    nativeBuildInputs
    buildInputs
    ;

  mesonFlags = [
    "--buildtype=debug"
    "-Dapp-id=de.capypara.FieldMonitor.Devel"
  ];

  postInstall = ''
    wrapProgram $out/bin/de.capypara.FieldMonitor.Devel --set RUST_LOG 'field_monitor=debug,libfieldmonitor=debug,oo7=debug,rdw=debug,rdw-vnc=debug,rdw-spice=debug,rdw-rdp=debug,vte=debug,GLib=info,warning' --set RUST_BACKTRACE '1'
  '';

  meta = with lib; {
    description = "XXXXXXXXXXXXXXX";
    homepage = "https://github.com/theCapypara/FieldMonitor";
    license = licenses.gpl3Plus;
    mainProgram = "de.capypara.FieldMonitor.Devel";
  };
}
