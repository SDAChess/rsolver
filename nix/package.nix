{
  lib,
  rustPlatform,
  pkg-config,
  aws-lc,
  sqlite,
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

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [
    aws-lc
    sqlite
  ];

  meta = {
    license = lib.licenses.asl20;
    mainProgram = "procxy";
  };
}
