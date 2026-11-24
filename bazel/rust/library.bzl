load("//rust:providers.bzl", "RustCrateInfo")

_TOOLCHAIN = Label("//rust:toolchain_type")

def _rust_library_impl(ctx):
    toolchain = ctx.toolchains[_TOOLCHAIN]
    crate_name = ctx.label.name
    rlib = ctx.actions.declare_file("lib{}.rlib".format(crate_name))
    deps = [dep[RustCrateInfo] for dep in ctx.attr.deps]

    args = ctx.actions.args()
    args.add("--crate-name", crate_name)
    args.add("--crate-type", "rlib")
    args.add("--edition", ctx.attr.edition)
    args.add("--emit", rlib, format = "link=%s")
    for dep in deps:
        args.add("--extern", dep.rlib, format = dep.crate_name + "=%s")
    args.add(ctx.file.crate_root)

    ctx.actions.run(
        executable = toolchain.rustc,
        arguments = [args],
        inputs = depset(
            [ctx.file.crate_root] + ctx.files.srcs + [dep.rlib for dep in deps],
            transitive = [toolchain.sysroot],
        ),
        outputs = [rlib],
        mnemonic = "Rustc",
    )
    return [
        DefaultInfo(files = depset([rlib])),
        RustCrateInfo(rlib = rlib, crate_name = crate_name),
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
        "srcs": attr.label_list(allow_files = [".rs"]),
    },
    toolchains = [_TOOLCHAIN],
)
