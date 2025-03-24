{
  mkShell,
  lib,
  stdenv,

  zlib,
  usbredir,
  gst_all_1,
  gtk-vnc,
  freerdp,
  spice-protocol,
  spice-gtk,
  libepoxy,
  llvmPackages_latest,
  glib,
  glibc,
  wrapGAppsHook4,
  gobject-introspection,
  desktop-file-utils,
  appstream,

  gtk4,
  vte-gtk4,
  libadwaita,
  meson,
  ninja,
  cmake,
  flatpak-builder,
  python312,
  libvirt,
  clang,
  rustup,
  openssl,
  pkg-config,
  python312Packages,
  libGL,

  editorconfig-checker,
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

  overrides = (builtins.fromTOML (builtins.readFile ../../rust-toolchain.toml));
  extraLibs = [
    stdenv.cc.cc.lib
    zlib
    usbredir
    gst_all_1.gstreamer
    gst_all_1.gst-plugins-base
    gst_all_1.gst-plugins-good
    patched-gtk-vnc
    freerdp
    spice-protocol
    spice-gtk
    libepoxy
    libGL
  ];
  libPath = lib.makeLibraryPath extraLibs;
in
mkShell {
  RUSTC_VERSION = overrides.toolchain.channel;
  # https://github.com/rust-lang/rust-bindgen#environment-variables
  LIBCLANG_PATH = lib.makeLibraryPath [ llvmPackages_latest.libclang.lib ];
  shellHook = ''
    export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
    export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
  '';
  # Add precompiled library to rustc search path
  RUSTFLAGS = (
    builtins.map (a: ''-L ${a}/lib'') [
      # add libraries here (e.g. pkgs.libvmi)
    ]
  );
  LD_LIBRARY_PATH = libPath;
  # Add glibc, clang, glib, and other headers to bindgen search path
  BINDGEN_EXTRA_CLANG_ARGS =
    # Includes normal include path
    (builtins.map (a: ''-I"${a}/include"'') [
      # add dev libraries here (e.g. pkgs.libvmi.dev)
      glibc.dev
    ])
    # Includes with special directory paths
    ++ [
      ''-I"${llvmPackages_latest.libclang.lib}/lib/clang/${llvmPackages_latest.libclang.version}/include"''
      ''-I"${glib.dev}/include/glib-2.0"''
      ''-I${glib.out}/lib/glib-2.0/include/''
    ];

  nativeBuildInputs = [
    wrapGAppsHook4
    gobject-introspection
  ];

  buildInputs =
    [
      gtk4
      vte-gtk4
      libadwaita
      libadwaita.devdoc
      meson
      ninja
      cmake
      zlib
      usbredir
      gst_all_1.gstreamer
      gst_all_1.gst-plugins-base
      gst_all_1.gst-plugins-good
      patched-gtk-vnc
      freerdp
      spice-protocol
      spice-gtk
      libepoxy
      flatpak-builder
      python312
      libvirt
      libGL
      desktop-file-utils
      appstream
    ]
    ++ (with python312Packages; [ pygobject3 ])
    ## RUST
    ++ [
      clang
      llvmPackages_latest.bintools
      rustup
      openssl
      pkg-config
    ]
    ## UTIL
    ++ [
      editorconfig-checker
    ];
}
