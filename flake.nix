{
  description = "Field Monitor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/master"; # TODO: Switch to nixos-25.11
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
      in
      {
        packages = rec {
          field-monitor = pkgs.callPackage ./build-aux/nix/pkg.nix { };
          field-monitor-devel = pkgs.callPackage ./build-aux/nix/pkg-devel.nix { inherit field-monitor; };
          default = field-monitor;
        };

        devShells = {
          default = pkgs.callPackage ./build-aux/nix/shell.nix { };
        };
      }
    );
}
