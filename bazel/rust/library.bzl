load("//rust:providers.bzl", "RustCrateInfo", "RustProcMacroInfo")
load("//rust:rustc.bzl", "TOOLCHAIN", "rustc_compile")

def _rust_library_impl(ctx):
    crate_name = ctx.label.name
    rlib = ctx.actions.declare_file("lib{}.rlib".format(crate_name))
    dep_crates = rustc_compile(ctx, "rlib", rlib)
    return [
        DefaultInfo(files = depset([rlib])),
        RustCrateInfo(
            rlib = rlib,
            crate_name = crate_name,
            transitive_crates = depset([rlib], transitive = [dep_crates]),
        ),
    ]

rust_library = rule(
    implementation = _rust_library_impl,
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
