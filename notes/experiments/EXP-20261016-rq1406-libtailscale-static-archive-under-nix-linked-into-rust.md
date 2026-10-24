---
id: EXP-20261016-rq1406
previous_id: manthan/EXP-0006
type: experiment
schema_version: 1
title: libtailscale static archive under Nix linked into Rust
project: PRJ-0001
status: concluded           # planned | active | paused | concluded | abandoned
outcome: positive           # null | positive | negative | inconclusive | infra
influenced_by: []
created: 2026-10-16
author: ank
protocols: []
materials: []
tags: [networking, phase-0, tsnet, nix]
review: approved
---

## Question
Can libtailscale's static archive be built with `buildGoModule` and linked into a Rust
binary under Nix on aarch64-darwin and x86_64-linux? Phase 0 spike 6.

## Hypothesis and predictions
A Go derivation with a pinned `vendorHash` produces `libtailscale.a` and `tailscale.h`
with `-buildmode=c-archive`; a Rust crate links it with `cargo:rustc-link-lib=static`
plus CoreFoundation and Security on darwin. The `libtailscale-sys` crate's own build
script runs `go build` and cannot work in the sandbox, so either patch it to accept a
prebuilt path or write a thin sys crate by hand.

## Plan
1. `spikes/06-libtailscale/` with its own flake: a Go derivation for libtailscale at a
   pinned commit, and a Rust package that calls `tailscale_new`, `tailscale_set_dir`,
   `tailscale_errmsg` and `tailscale_close` with no network.
2. `nix build` on darwin; then on linux through the remote builder.
3. Record build time, archive size, the link flags needed, and whether the vendored
   crate could be reused.

## Success criteria
- `nix build` succeeds on both systems and the binary runs.
- The link flags and derivation shape are documented for the real node and hub crates.

## Findings

Both targets built and ran on the first link attempt. Code in
`code/rc2s2-a-remote-command-and-control-session-server/spikes/06-libtailscale/`.

- **libtailscale** `d1dc45d039edb7759ecbee6ba2aea0c4bca884b8` (main, 2026-10-16) needs
  Go 1.27.1 or newer (`go.mod`), so the derivation is `buildGo127Module` (1.27.2 at the
  project's nixpkgs `55bfb4d7`); the default `go` (1.26.8) would refuse it. The module
  is `package main` with cgo exports, so `buildPhase` and `installPhase` are replaced
  with `go build -buildvcs=false -buildmode=c-archive -o libtailscale.a .` and a plain
  `install` of `libtailscale.a`, `tailscale.h` (public API) and the cgo-generated
  `libtailscale.h`. `doCheck = false`. `env.CGO_ENABLED = "1"`.
- Hashes: src `sha256-BsCfPZbxAkOFMnohDxLSmiY2FzAwAHGdzakmivVboWQ=`, vendorHash
  `sha256-1Y21dNgCNMfVQvZN6k66GHvgdZo8lle5J8B5UES2Ryg=`. The vendored module set is the
  whole of `tailscale.com v1.104.1` and its tree (the module fetch took about a minute
  on darwin).
- Times and sizes: Go build 37 s on aarch64-darwin (47 s whole derivation); the Rust
  derivation 21 s with `cargo build --release` at 5 s. Through the remote builder the
  whole linux chain, closure copy and vendoring included, took 2 m 56 s. Archive
  26.8 MB on darwin, 30.3 MB on linux (both after nixpkgs' default `strip -S`);
  resulting binaries 22.7 MB and 28.0 MB.
- **Link flags**: `cargo:rustc-link-search=native=<libdir>` and
  `cargo:rustc-link-lib=static=tailscale`; on `apple-` targets additionally
  `cargo:rustc-link-lib=framework=CoreFoundation` and `framework=Security`. Nothing else
  on either OS. darwin `otool -L`: CoreFoundation, Security, libSystem. linux
  `patchelf --print-needed`: libgcc_s, libc, ld-linux. The nixpkgs darwin stdenv's SDK
  was enough; `-mmacos-version-min` from the upstream Makefile was not needed.
- The four calls behave as the header documents without any network:
  `tailscale_new` returns `2752513` (`42<<16 + 1`), `set_dir` 0, `errmsg` 0 with an
  empty message, `close` 0 on a never-started server, and `errmsg` after `close`
  returns `EBADF` (9). Same output on both targets. `tailscale_start` was never called.
- The remote builder worked unattended: `nix build .#checks.x86_64-linux.run` went to
  `ssh-ng://ank@spirit` with no flags.
- **The `libtailscale-sys` crate (messense/libtailscale-rs 0.2.2) was not reused.** Its
  `build.rs` runs `go build -buildmode=c-archive` against a vendored submodule and
  downloads modules, which cannot work in the sandbox; its only link output is the same
  three lines as above, so a hand-written `build.rs` of 15 lines replaces it. The
  prediction in the hypothesis held. The safe `libtailscale` wrapper crate on top of it
  could still be used if its `-sys` dependency is patched (`[patch.crates-io]`) to one
  that reads a prebuilt path; not tried here.

Recommended derivation shape for the real hub and node crates:

1. One `libtailscale` derivation in the project flake exactly as in the spike
   (`buildGo127Module`, pinned rev, `vendorHash`, c-archive build, `lib/` + `include/`).
   Bump the Go attribute together with the pinned rev when `go.mod` moves.
2. A workspace crate `rc2s2-tailscale-sys` whose `build.rs` reads `LIBTAILSCALE_LIB_DIR`
   and emits the flags above, with the `extern "C"` declarations hand-written from
   `tailscale.h` (19 functions, all `int`/`char*`/`size_t`; bindgen is not worth a
   libclang dependency). Panic with a clear message if the variable is missing.
3. The hub and node `buildRustPackage` (or crane) derivations set
   `env.LIBTAILSCALE_LIB_DIR = "${libtailscale}/lib"`. For `nix develop`, export the
   same variable in `shellHook` so plain `cargo build` works too.
4. Keep a `checks.<system>.run`-style smoke test that exercises
   `new/set_dir/errmsg/close` in the sandbox; it is cheap and catches link regressions
   on both OSes.
5. Android stays out of scope: the derivation builds for the host only; a cross-build
   would need `GOOS=android` with the NDK `CC`, which is a separate spike.

## Runs
- [[RUN-20261016-qqc0qw]] aarch64-darwin: build, link, run; all green
- [[RUN-20261016-qt9rk9]] x86_64-linux via remote builder spirit: build, link, run; all green

## Conclusion
- libtailscale's static archive can be built with `buildGoModule` and linked into a Rust
binary under Nix on aarch64-darwin and x86_64-linux.
