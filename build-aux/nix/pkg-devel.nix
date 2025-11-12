{
  stdenv,
  field-monitor,
  lib,
  vte-gtk4,
  gtk-vnc,
}:

let
  patched-gtk-vnc = (
    gtk-vnc.dev.overrideAttrs (
      finalAttrs: previousAttrs: {
        mesonBuildType = "debug";
        mesonFlags = previousAttrs.mesonFlags ++ [
          "-Ddebug=true"
        ];
        dontStrip = true;
        enableDebugging = true;
      }
    )
  );
  patched-vte-gtk4 = vte-gtk4.dev.overrideAttrs (
    finalAttrs: previousAttrs: {
      mesonBuildType = "debug";
      mesonFlags = previousAttrs.mesonFlags ++ [
        "-Ddebug=true"
      ];
      dontStrip = true;
      enableDebugging = true;
    }
  );
in

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

  buildInputs = field-monitor.prodBuildInputs ++ [
    patched-vte-gtk4
    patched-gtk-vnc
  ];

  mesonBuildType = "debug";
  mesonFlags = [
    "-Dapp-id=de.capypara.FieldMonitor.Devel"
  ];

  postInstall = ''
    wrapProgram $out/bin/de.capypara.FieldMonitor.Devel --prefix PATH ':' "$out/libexec" --set RUST_LOG 'field_monitor=debug,libfieldmonitor=debug,oo7=debug,rdw=debug,rdw-vnc=debug,rdw-spice=debug,rdw-rdp=debug,vte=debug,gtk-vnc=debug,Adwaita=info,GLib=info,warning' --set RUST_BACKTRACE '1'
  '';

  doCheck = true;

  meta = with lib; {
    description = "Viewer for virtual machines and other external screens";
    homepage = "https://github.com/theCapypara/field-monitor";
    license = licenses.gpl3Plus;
    mainProgram = "de.capypara.FieldMonitor.Devel";
  };
}
