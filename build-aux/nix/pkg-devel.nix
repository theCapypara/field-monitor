{
  stdenv,
  field-monitor-source,
  lib,
  vte-gtk4,
}:

stdenv.mkDerivation {
  pname = "field-monitor-devel";
  version = "47.0";

  inherit (field-monitor-source) src cargoDeps buildInputs;

  nativeBuildInputs = field-monitor-source.prodNativeBuildInputs ++ [
    (vte-gtk4.dev.overrideAttrs (
      finalAttrs: previousAttrs: {
        mesonFlags = previousAttrs.mesonFlags ++ [
          "--buildtype=debug"
          "-Ddebug=true"
        ];
        dontStrip = true;
      }
    ))
  ];

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
