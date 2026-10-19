#!/bin/sh
# Builds rc2s2-core for arm64 Android into the app's jniLibs and generates its Kotlin
# bindings into app/build/generated/uniffi. Run from the project root inside
# `nix develop .#android`; Gradle's buildCore task calls it before every build.
set -eu
cargo ndk -t arm64-v8a --platform 26 -o android/app/src/main/jniLibs build -p rc2s2-core --release --lib
cargo run --quiet -p uniffi-bindgen -- generate \
  --library target/aarch64-linux-android/release/librc2s2_core.so \
  --language kotlin --no-format --out-dir android/app/build/generated/uniffi
