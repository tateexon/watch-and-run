{
  description = "development shell";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = inputs@{ self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ ];
        };

        # Importing the shell environments from separate files
        fullEnv = pkgs.callPackage ./nix/devshell.nix {
          inherit pkgs;
        };

      in rec {
        devShell = fullEnv;

        formatter = pkgs.nixpkgs-fmt;
      });
}
