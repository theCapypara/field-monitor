{
  stdenv,
  field-monitor,
  lib,
  fetchgit,
  vte-gtk4,
  gtk-vnc,
}:

let
  patched-gtk-vnc = (
    gtk-vnc.dev.overrideAttrs (
      finalAttrs: previousAttrs: {
        version = "1.3.1+ca-dbg";
        dontStrip = true;
        enableDebugging = true;
        mesonFlags = previousAttrs.mesonFlags ++ [
          "--buildtype=debug"
          "-Ddebug=true"
        ];
        src = fetchgit {
          # see note in flatpak sources
          url = "https://gitlab.gnome.org/theCapypara/gtk-vnc.git";
          rev = "ad14f260652e07aa2e7fc7481b7d998855160d2d";
          hash = "sha256-5jXy0YMDrBSwlqMUS9NGDz5QLe2bi1LOUhLAQlTHhCI=";
        };
      }
    )
  );
  patched-vte-gtk4 = vte-gtk4.dev.overrideAttrs (
    finalAttrs: previousAttrs: {
      mesonFlags = previousAttrs.mesonFlags ++ [
        "--buildtype=debug"
        "-Ddebug=true"
      ];
      dontStrip = true;
      enableDebugging = true;
    }
  );
in

stdenv.mkDerivation {
  pname = "field-monitor-devel";
  version = "47.0";

  dontStrip = true;
  enableDebugging = true;

  inherit (field-monitor) src cargoDeps buildInputs;

  nativeBuildInputs = field-monitor.prodNativeBuildInputs ++ [
    patched-vte-gtk4
    patched-gtk-vnc
  ];

  mesonFlags = [
    "--buildtype=debug"
    "-Dapp-id=de.capypara.FieldMonitor.Devel"
  ];

  postInstall = ''
    wrapProgram $out/bin/de.capypara.FieldMonitor.Devel --prefix PATH ':' "$out/bin" --set RUST_LOG 'field_monitor=debug,libfieldmonitor=debug,oo7=debug,rdw=debug,rdw-vnc=debug,rdw-spice=debug,rdw-rdp=debug,vte=debug,gtk-vnc=debug,Adwaita=info,GLib=info,warning' --set RUST_BACKTRACE '1'
  '';

  meta = with lib; {
    description = "XXXXXXXXXXXXXXX";
    homepage = "https://github.com/theCapypara/field-monitor";
    license = licenses.gpl3Plus;
    mainProgram = "de.capypara.FieldMonitor.Devel";
  };
}
