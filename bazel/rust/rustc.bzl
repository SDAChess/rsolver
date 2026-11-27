load("//rust:providers.bzl", "RustCrateInfo")

TOOLCHAIN = Label("//rust:toolchain_type")

def _dirname(file):
    return file.dirname

def rustc_compile(ctx, crate_type, output):
    """Compiles the crate described by ctx.attr into output.

    Args:
      ctx: the rule context; reads crate_root, srcs, deps and edition.
      crate_type: rustc's --crate-type; "bin" also passes the linker.
      output: the File rustc emits.

    Returns:
        depset of Files: the transitive rlibs of the crate's deps.
    """
    toolchain = ctx.toolchains[TOOLCHAIN]
    deps = [dep[RustCrateInfo] for dep in ctx.attr.deps]
    dep_rlibs = depset(transitive = [dep.transitive_rlibs for dep in deps])
    direct_inputs = [ctx.file.crate_root] + ctx.files.srcs

    args = ctx.actions.args()
    args.add("--crate-name", ctx.label.name)
    args.add("--crate-type", crate_type)
    args.add("--edition", ctx.attr.edition)

    # Crates with the same name (two versions, or two packages) need distinct identities.
    args.add("-C", "metadata=" + str(ctx.label))
    if crate_type == "bin":
        args.add("-C", toolchain.linker, format = "linker=%s")
        direct_inputs.append(toolchain.linker)
    args.add("--emit", output, format = "link=%s")
    for dep in deps:
        args.add("--extern", dep.rlib, format = dep.crate_name + "=%s")
    args.add_all(
        dep_rlibs,
        map_each = _dirname,
        uniquify = True,
        before_each = "-L",
        format_each = "dependency=%s",
    )
    args.add(ctx.file.crate_root)

    ctx.actions.run(
        executable = toolchain.rustc,
        arguments = [args],
        inputs = depset(
            direct_inputs,
            transitive = [toolchain.sysroot, dep_rlibs],
        ),
        outputs = [output],
        mnemonic = "Rustc",
    )
    return dep_rlibs
