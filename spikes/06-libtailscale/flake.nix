# EXP-0006: libtailscale as a static c-archive from a Go derivation, linked into Rust.
# Throwaway spike; standalone flake pinned to the same nixpkgs rev as the project flake.
{
  description = "EXP-0006 libtailscale c-archive under Nix linked into Rust";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/55bfb4d7437bd4e09ab7cba58a43bd68eb310044";

  outputs = { self, nixpkgs }:
    let
      systems = [ "aarch64-darwin" "x86_64-darwin" "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAll (pkgs: rec {
        # go.mod says `go 1.27.1`; the default go in this nixpkgs is 1.26.8.
        libtailscale = pkgs.buildGo127Module {
          pname = "libtailscale";
          version = "0-unstable-2026-10-16";
          src = pkgs.fetchFromGitHub {
            owner = "tailscale";
            repo = "libtailscale";
            rev = "d1dc45d039edb7759ecbee6ba2aea0c4bca884b8";
            hash = "sha256-BsCfPZbxAkOFMnohDxLSmiY2FzAwAHGdzakmivVboWQ=";
          };
          vendorHash = "sha256-1Y21dNgCNMfVQvZN6k66GHvgdZo8lle5J8B5UES2Ryg=";
          env.CGO_ENABLED = "1";
          # The module is `package main` with cgo exports; buildGoModule's default
          # build/install phases expect executables, so both are replaced.
          buildPhase = ''
            runHook preBuild
            go build -buildvcs=false -buildmode=c-archive -o libtailscale.a .
            runHook postBuild
          '';
          installPhase = ''
            runHook preInstall
            install -Dm644 libtailscale.a $out/lib/libtailscale.a
            # tailscale.h is the public API; libtailscale.h is the cgo-generated header.
            install -Dm644 tailscale.h $out/include/tailscale.h
            install -Dm644 libtailscale.h $out/include/libtailscale.h
            runHook postInstall
          '';
          doCheck = false;
        };

        default = pkgs.rustPlatform.buildRustPackage {
          pname = "ts-hello";
          version = "0.1.0";
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [ ./Cargo.toml ./Cargo.lock ./build.rs ./src ];
          };
          cargoLock.lockFile = ./Cargo.lock;
          # build.rs reads this; nothing runs `go` on the Rust side.
          env.LIBTAILSCALE_LIB_DIR = "${libtailscale}/lib";
          doCheck = false;
        };
      });

      checks = forAll (pkgs: {
        # Runs the binary in the sandbox (no network) and keeps its output.
        run = pkgs.runCommand "ts-hello-run" { } ''
          ${self.packages.${pkgs.stdenv.hostPlatform.system}.default}/bin/ts-hello | tee $out
        '';
      });
    };
}
