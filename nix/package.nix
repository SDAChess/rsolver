{
  lib,
  rustPlatform,
}:
let
  manifest = (lib.importTOML ../Cargo.toml).workspace.package;
in
rustPlatform.buildRustPackage {
  pname = "procxy";
  inherit (manifest) version;

  __structuredAttrs = true;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../procxy
      ../rsolver
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;
  cargoBuildFlags = [
    "--package"
    "procxy"
  ];

  meta = {
    license = lib.licenses.asl20;
    mainProgram = "procxy";
  };
}
