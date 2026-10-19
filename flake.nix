{
  description = "rc2s2 - A remote command and control session server (PRJ-0002)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "aarch64-darwin" "x86_64-darwin" "x86_64-linux" "aarch64-linux" ];
      perSystem = { pkgs, lib, ... }: {
        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.python3
            pkgs.cargo
            pkgs.rustc
            pkgs.clippy
            pkgs.rustfmt
            pkgs.rust-analyzer
            pkgs.protobuf
          ];
          PROTOC = "${pkgs.protobuf}/bin/protoc";
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };

        # The cargo workspace, built and tested; spikes/ and docs/ stay out of the source.
        checks.workspace = pkgs.rustPlatform.buildRustPackage {
          pname = "rc2s2";
          version = "0.1.0";
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [ ./Cargo.toml ./Cargo.lock ./crates ];
          };
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.protobuf ];
          PROTOC = "${pkgs.protobuf}/bin/protoc";
        };
      };
    };
}
