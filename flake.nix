{
  description = "A devShell example";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      {
        devShells.default = with pkgs; mkShell {
          packages = [
            rust-analyzer
            rustfmt
          ];

          buildInputs = [
            openssl
            pkg-config
            rustup
            rust-bin.nightly.latest.default
          ];

          shellHook = ''
            # alias ls=eza
            # alias find=fd
          '';
        };
      }
    );
}