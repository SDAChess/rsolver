# rsolver

> [!WARNING]
> rsolver is in early alpha and not bootstrapped yet.

A very opinionated take on Building Rust.

rsolver resolves crates.io dependencies for build systems other than cargo,
starting with Bazel. It solves versions with
[pubgrub](https://github.com/pubgrub-rs/pubgrub) and writes a lockfile that
describes the full build graph. It never downloads a crate and never runs a
build script.

## Who this is for

rsolver is for teams that:

- don't publish crates. Publishing to crates.io still needs cargo and a
  `Cargo.toml`.
- build Rust alongside other languages in a multi-language build system such as
  Bazel.
- want fine-grained control over their build, and will spend time writing
  fixups to keep the build graph exact and hermetic.

## Why

### Per-binary build graphs

Cargo does not report which crates, versions and features went into a given
binary. `Cargo.lock` covers the whole workspace and every platform, and cargo
unifies features across everything it builds in one run. SBOM and supply-chain
tools have to reconstruct the answer from `cargo metadata` or `cargo tree`.

With rsolver, each binary lists the features it needs. The build system walks
the lockfile from that list and compiles only the crates and features the binary
uses. Host builds (proc-macros) and target builds use separate feature sets.
What you build is exactly what you report.

### No build scripts

Build scripts run arbitrary code at build time, and their output depends on the
machine. rsolver never runs them, including first-party ones. Each crate with a
build script needs a fixup: a short declaration of what the script would
produce, such as `--cfg` flags or a generated file. A crate without a fixup
fails to build.

### No downloads during resolution

Cargo downloads every crate during feature resolution, because the registry
index does not mark proc-macros
([cargo#15834](https://github.com/rust-lang/cargo/issues/15834)). rsolver reads
the index and gets the missing facts from procxy. The build system downloads
only the crates a build needs.

## Contents

- `rsolver/`: the resolver library. It takes root requirements, allowed
  features and target platforms, and returns the locked graph. It contains no
  Bazel code, so you can use it to integrate Rust with other build systems.
- `procxy/`: a registry proxy that serves the per-version facts the index
  lacks: proc-macro or not, build script or not, edition, lib target.
- `bazel/`: `rules_oxide`, the Bazel rules for Rust (`rust_library`,
  `rust_binary`, `rust_proc_macro`). They use rsolver for crates.io
  dependencies: you declare crates in `MODULE.bazel`, a pin target calls
  rsolver and writes the lockfile, and builds only read the checked-in
  lockfile.

## Development

```sh
nix develop
cargo test
cd bazel && bazel test //tests:binaries
nix fmt
```

## License

Apache-2.0
