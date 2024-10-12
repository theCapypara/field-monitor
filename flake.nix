{
  description = "Field Monitor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/gnome";
    flake-utils.url = "github:numtide/flake-utils";
    systems.url = "github:nix-systems/default";
  };

  outputs =
    {
      self,
      nixpkgs,
      systems,
      flake-utils,
    }:
    flake-utils.lib.eachSystem (import systems) (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        mkFieldMonitorWrapped = import ./build-aux/nix/_mkFieldMonitorWrapped.nix {
          inherit (pkgs) writeShellApplication;
        };
        field-monitor-source = pkgs.callPackage ./build-aux/nix/pkg.nix { };
      in
      {
        packages = rec {
          field-monitor = mkFieldMonitorWrapped field-monitor-source;
          field-monitor-devel = mkFieldMonitorWrapped (
            pkgs.callPackage ./build-aux/nix/pkg-devel.nix { inherit field-monitor-source; }
          );
          default = field-monitor;
        };

        devShells = {
          default = pkgs.callPackage ./build-aux/nix/shell.nix { };
        };
      }
    );
}
