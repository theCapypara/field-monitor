{
  description = "Field Monitor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    {
      self,
      nixpkgs,
    }:
    let
      eachSystem = nixpkgs.lib.genAttrs [
        "aarch64-linux"
        "x86_64-linux"
      ];
      eachSystemPkgs =
        fn:
        builtins.mapAttrs fn (
          eachSystem (
            system:
            #todo, uncomment when overlay gone: nixpkgs.legacyPackages.${system} or
            (import nixpkgs {
              inherit system;
              overlays = [
                (final: prev: {
                  spice-glib = final.callPackage ./build-aux/nix/spice-glib.nix { };
                })
              ];
            })
          )
        );
      eachPkgs = fn: eachSystemPkgs (_: fn);
    in
    {
      formatter = eachPkgs (pkgs: pkgs.nixfmt-tree);

      packages = eachPkgs (pkgs: rec {
        field-monitor = pkgs.callPackage ./build-aux/nix/pkg.nix { };
        field-monitor-devel = pkgs.callPackage ./build-aux/nix/pkg-devel.nix { inherit field-monitor; };
        # Field Monitor development build using a locally checked out rdw version.
        # This requires an env variable & impure build, see `pkg-devel-local-rdw.nix` for details.
        field-monitor-devel-local-rdw = pkgs.callPackage ./build-aux/nix/pkg-devel-local-rdw.nix {
          inherit field-monitor-devel;
        };
        default = field-monitor;
      });

      checks = eachPkgs (pkgs: rec {
        field-monitor = pkgs.callPackage ./build-aux/nix/pkg.nix { };
        field-monitor-devel = pkgs.callPackage ./build-aux/nix/pkg-devel.nix { inherit field-monitor; };
      });

      devShells = eachPkgs (pkgs: {
        default = pkgs.callPackage ./build-aux/nix/shell.nix { };
      });
    };
}
