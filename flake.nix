{
  description = "rc2s2 - A remote command and control session server (PRJ-0002)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";
    # A Rust toolchain with the Android targets' standard library, which nixpkgs' rustc
    # lacks; used by the android shell only (ADR-0024).
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "aarch64-darwin" "x86_64-darwin" "x86_64-linux" "aarch64-linux" ];
      perSystem = { system, lib, ... }:
        let
          # The Android SDK is unfree: its licence is accepted here, and only its components
          # may be unfree, under the names androidenv gives them (ADR-0024).
          androidComponents = [ "build-tools" "cmake" "cmdline-tools" "emulator" "ndk"
                                "platform-tools" "platforms" "tools" ];
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ inputs.rust-overlay.overlays.default ];
            config = {
              android_sdk.accept_license = true;
              allowUnfreePredicate = pkg:
                let n = lib.getName pkg; in
                lib.hasPrefix "android-sdk-" n || lib.hasPrefix "system-image-" n
                || builtins.elem n androidComponents;
            };
          };
          android = pkgs.androidenv.composeAndroidPackages {
            platformVersions = [ "35" ];
            buildToolsVersions = [ "35.0.0" ];
            includeNDK = true;
            includeEmulator = true;
            includeSystemImages = true;
            systemImageTypes = [ "google_apis" ];
            abiVersions = [ "arm64-v8a" ];
          };
          sdk = "${android.androidsdk}/libexec/android-sdk";
          rustAndroid = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "clippy" "rustfmt" ];
            targets = [ "aarch64-linux-android" ];
          };

          # The workspace source, shared by the checks and the binary packages:
          # spikes/, docs/ and android/ stay out of it.
          rc2s2Src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [ ./Cargo.toml ./Cargo.lock ./crates ];
          };
          # One release binary of the workspace. `bin` is the binary name; the
          # node's `rc2s2` and the hub's `rc2s2-hub` are each built on their own so
          # the closure holds only that binary's dependency graph (the uniffi core
          # and bindgen crates never enter the hub or node). Tests are skipped: the
          # hub's suite spins up a node and the node's needs a PTY; the workspace
          # check already builds and tests everything.
          mkBin = pname: bin: pkgs.rustPlatform.buildRustPackage {
            inherit pname;
            version = "0.1.0";
            src = rc2s2Src;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.protobuf ];
            PROTOC = "${pkgs.protobuf}/bin/protoc";
            cargoBuildFlags = [ "--bin" bin ];
            doCheck = false;
            meta.mainProgram = bin;
          };
        in
        {
          _module.args.pkgs = pkgs;

          # The deployable binaries, consumed by the hub (NixOS) and node
          # (home-manager) modules in the owner's neusis fleet config.
          packages = {
            rc2s2-hub = mkBin "rc2s2-hub" "rc2s2-hub";
            rc2s2 = mkBin "rc2s2" "rc2s2";
          };

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

          # The Android app and the client core for Android: the SDK with an arm64 emulator
          # image, the NDK, cargo-ndk, Gradle and a JDK. Several GB, so it is a shell of its
          # own and the default shell and the checks stay light.
          devShells.android = pkgs.mkShell {
            packages = [
              rustAndroid
              pkgs.cargo-ndk
              pkgs.protobuf
              pkgs.gradle
              pkgs.jdk17
              android.androidsdk
              # A local ntfy for push in development (ADR-0025).
              pkgs.ntfy-sh
            ];
            PROTOC = "${pkgs.protobuf}/bin/protoc";
            ANDROID_HOME = sdk;
            ANDROID_SDK_ROOT = sdk;
            ANDROID_NDK_HOME = "${sdk}/ndk/${android.ndk-bundle.version}";
            JAVA_HOME = pkgs.jdk17.home;
            # Emulator devices and Android tool state live outside the repo and the store.
            shellHook = ''
              export ANDROID_USER_HOME="''${XDG_STATE_HOME:-$HOME/.local/state}/rc2s2/android"
              export ANDROID_AVD_HOME="$ANDROID_USER_HOME/avd"
              export GRADLE_USER_HOME="''${XDG_STATE_HOME:-$HOME/.local/state}/rc2s2/gradle"
              mkdir -p "$ANDROID_AVD_HOME"
            '';
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
