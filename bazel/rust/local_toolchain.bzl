def _local_rust_toolchain_impl(rctx):
    rustc = rctx.getenv("RUSTC")
    if not rustc:
        fail("RUSTC is not set; run bazel from the nix devShell")
    sysroot = rctx.execute([rustc, "--print", "sysroot"]).stdout.strip()
    rctx.symlink(sysroot, "sysroot")
    rctx.template("BUILD.bazel", Label("//rust:BUILD.local_toolchain.bazel"))

local_rust_toolchain = repository_rule(
    implementation = _local_rust_toolchain_impl,
    configure = True,
)
