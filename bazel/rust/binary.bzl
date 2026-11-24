load("//rust:providers.bzl", "RustCrateInfo", "RustProcMacroInfo")
load("//rust:rustc.bzl", "TOOLCHAIN", "rustc_compile")

def _rust_binary_impl(ctx):
    binary = ctx.actions.declare_file(ctx.label.name)
    rustc_compile(ctx, "bin", binary)
    return [DefaultInfo(executable = binary)]

rust_binary = rule(
    implementation = _rust_binary_impl,
    attrs = {
        "crate_root": attr.label(allow_single_file = [".rs"], mandatory = True),
        "deps": attr.label_list(providers = [RustCrateInfo]),
        "edition": attr.string(
            mandatory = True,
            values = ["2015", "2018", "2021", "2024"],
        ),
        # Proc-macros run inside rustc, so they and their deps build for the exec platform.
        "proc_macro_deps": attr.label_list(cfg = "exec", providers = [RustProcMacroInfo]),
        "srcs": attr.label_list(allow_files = [".rs"]),
    },
    executable = True,
    toolchains = [TOOLCHAIN],
)
