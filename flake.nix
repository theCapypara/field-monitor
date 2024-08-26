{
  description = "Development Environment for Field Monitor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
  };

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forEachSupportedSystem =
        f: nixpkgs.lib.genAttrs supportedSystems (system: f { pkgs = import nixpkgs { inherit system; }; });
    in
    {
      devShells = forEachSupportedSystem (
        { pkgs }:
        let
          overrides = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml));
          extraLibs = with pkgs; [
            stdenv.cc.cc.lib
            zlib
            usbredir
            gst_all_1.gstreamer
            gst_all_1.gst-plugins-base
            gst_all_1.gst-plugins-good
            gtk-vnc
            libepoxy
          ];
          libPath = with pkgs; lib.makeLibraryPath extraLibs;
          gvnc = pkgs.callPackage ./nix/package/gvnc.nix { };
        in
        {
          default = pkgs.mkShell {
            RUSTC_VERSION = overrides.toolchain.channel;
            # https://github.com/rust-lang/rust-bindgen#environment-variables
            LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_latest.libclang.lib ];
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
                pkgs.glibc.dev
              ])
              # Includes with special directory paths
              ++ [
                ''-I"${pkgs.llvmPackages_latest.libclang.lib}/lib/clang/${pkgs.llvmPackages_latest.libclang.version}/include"''
                ''-I"${pkgs.glib.dev}/include/glib-2.0"''
                ''-I${pkgs.glib.out}/lib/glib-2.0/include/''
              ];

            nativeBuildInputs = with pkgs; [
              wrapGAppsHook
              gobject-introspection
            ];

            buildInputs =
              with pkgs;
              [
                gtk4
                libadwaita
                meson
                ninja
                cmake
                zlib
                usbredir
                gst_all_1.gstreamer
                gst_all_1.gst-plugins-base
                gst_all_1.gst-plugins-good
                gtk-vnc
                libepoxy
                flatpak-builder
              ]
              ## RUST
              ++ [
                clang
                llvmPackages_12.bintools
                rustup
                openssl
                pkg-config
              ];
          };
        }
      );
    };
}
