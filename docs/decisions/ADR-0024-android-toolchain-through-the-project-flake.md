# ADR-0024: The Android toolchain comes from the project flake, in a shell of its own

Status: accepted
Date: 2026-10-19

## Context
Step 5 builds the client core for Android and the Compose app. The Android SDK and NDK in
nixpkgs are unfree, and nixpkgs' rustc has no standard library for Android targets. The
alternatives were Android Studio on the host, outside Nix, or the whole toolchain in the
flake; the owner chose the flake and accepted the SDK licence, with an emulator on the Mac
for development. The SDK with an emulator image runs to several GB, and the root
`nix flake check` builds every project's default shell.

## Decision
`devShells.android` in the project flake: `androidenv.composeAndroidPackages` (platform and
build tools 35, the NDK, the emulator, the `google_apis` arm64-v8a system image), cargo-ndk,
Gradle, JDK 17, and a Rust toolchain from `rust-overlay` with the `aarch64-linux-android`
target. The flake's nixpkgs accepts the Android SDK licence and allows unfree packages only
under the names androidenv gives the SDK's components. The default shell and
`checks.workspace` keep nixpkgs' Rust and stay free of the SDK. Emulator devices and
Android tool state live under `$XDG_STATE_HOME/rc2s2/android`, outside the repo.

## Consequences
`nix develop .#android` fetches the SDK on first use; the root check does not. Two Rust
toolchains exist (nixpkgs' in the default shell, rust-overlay's in the android one), so a
version difference between them is possible. Kotlin bindings come from the
`uniffi-bindgen` crate, kept apart so the generator stays out of the library the app
loads. Only arm64 Android is built for now.
