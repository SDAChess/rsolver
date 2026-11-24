_TOOLCHAIN = Label("//rust:toolchain_type")

def _rust_library_impl(ctx):
    toolchain = ctx.toolchains[_TOOLCHAIN]
    rlib = ctx.actions.declare_file("lib{}.rlib".format(ctx.label.name))

    args = ctx.actions.args()
    args.add("--crate-name", ctx.label.name)
    args.add("--crate-type", "rlib")
    args.add("--edition", ctx.attr.edition)
    args.add("--emit", rlib, format = "link=%s")
    args.add(ctx.file.crate_root)

    ctx.actions.run(
        executable = toolchain.rustc,
        arguments = [args],
        inputs = depset([ctx.file.crate_root], transitive = [toolchain.sysroot]),
        outputs = [rlib],
        mnemonic = "Rustc",
    )
    return [DefaultInfo(files = depset([rlib]))]

rust_library = rule(
    implementation = _rust_library_impl,
    attrs = {
        "crate_root": attr.label(allow_single_file = [".rs"], mandatory = True),
        "edition": attr.string(
            mandatory = True,
            values = ["2015", "2018", "2021", "2024"],
        ),
    },
    toolchains = [_TOOLCHAIN],
)
