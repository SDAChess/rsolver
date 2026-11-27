load("//rust:providers.bzl", "RustCrateInfo", "RustProcMacroInfo")
load("//rust:rustc.bzl", "TOOLCHAIN", "rustc_compile")

def _rust_proc_macro_impl(ctx):
    crate_name = ctx.label.name
    so = ctx.actions.declare_file("lib{}.so".format(crate_name))
    rustc_compile(ctx, "proc-macro", so)
    return [
        DefaultInfo(files = depset([so])),
        RustProcMacroInfo(so = so, crate_name = crate_name),
    ]

rust_proc_macro = rule(
    implementation = _rust_proc_macro_impl,
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
    toolchains = [TOOLCHAIN],
)
