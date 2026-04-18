{
  description = "Field Monitor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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
          # Field Monitor development built using a locally checked out rdw version.
          # This requires an env variable & impure build, see `pkg-devel-local-rdw.nix` for details.
          field-monitor-devel-local-rdw = pkgs.callPackage ./build-aux/nix/pkg-devel-local-rdw.nix { inherit field-monitor-devel; };
          default = field-monitor;
        };

        devShells = {
          default = pkgs.callPackage ./build-aux/nix/shell.nix { };
        };
      }
    );
}
