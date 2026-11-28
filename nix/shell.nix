{
  mkShellNoCC,
  bazel_9,
  stdenv,
  procxy,
  rustToolchain,
}:
mkShellNoCC {
  inputsFrom = [ procxy ];
  packages = [
    rustToolchain
    bazel_9
  ];
  RUSTC = "${rustToolchain}/bin/rustc";
  RUST_LINKER = "${stdenv.cc}/bin/cc";
}
