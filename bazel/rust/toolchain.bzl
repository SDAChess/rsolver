def _rust_toolchain_impl(ctx):
    return [platform_common.ToolchainInfo(
        linker = ctx.file.linker,
        rustc = ctx.file.rustc,
        sysroot = depset(ctx.files.sysroot),
    )]

rust_toolchain = rule(
    implementation = _rust_toolchain_impl,
    attrs = {
        "linker": attr.label(allow_single_file = True, mandatory = True),
        "rustc": attr.label(allow_single_file = True, mandatory = True),
        "sysroot": attr.label_list(allow_files = True, mandatory = True),
    },
)
