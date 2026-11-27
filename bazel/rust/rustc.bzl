load("//rust:providers.bzl", "RustCrateInfo", "RustProcMacroInfo")

TOOLCHAIN = Label("//rust:toolchain_type")

def _dirname(file):
    return file.dirname

def rustc_compile(ctx, crate_type, output):
    """Compiles the crate described by ctx.attr into output.

    Args:
      ctx: the rule context; reads crate_root, srcs, deps, proc_macro_deps and edition.
      crate_type: rustc's --crate-type; every type but "rlib" also passes the linker.
      output: the File rustc emits.

    Returns:
        depset of Files: the transitive crates of the crate's deps and proc-macros.
    """
    toolchain = ctx.toolchains[TOOLCHAIN]
    deps = [dep[RustCrateInfo] for dep in ctx.attr.deps]
    proc_macros = [dep[RustProcMacroInfo] for dep in ctx.attr.proc_macro_deps]

    # Downstream crates need the proc-macros too: rustc loads every crate a dep depends on.
    dep_crates = depset(
        [pm.so for pm in proc_macros],
        transitive = [dep.transitive_crates for dep in deps],
    )
    direct_inputs = [ctx.file.crate_root] + ctx.files.srcs

    args = ctx.actions.args()
    args.add("--crate-name", ctx.label.name)
    args.add("--crate-type", crate_type)
    args.add("--edition", ctx.attr.edition)

    # Crates with the same name (two versions, or two packages) need distinct identities.
    args.add("-C", "metadata=" + str(ctx.label))
    if crate_type != "rlib":
        args.add("-C", toolchain.linker, format = "linker=%s")
        direct_inputs.append(toolchain.linker)
    if crate_type == "proc-macro":
        args.add("--extern", "proc_macro")
    args.add("--emit", output, format = "link=%s")
    for dep in deps:
        args.add("--extern", dep.rlib, format = dep.crate_name + "=%s")
    for pm in proc_macros:
        args.add("--extern", pm.so, format = pm.crate_name + "=%s")
    args.add_all(
        dep_crates,
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
            transitive = [toolchain.sysroot, dep_crates],
        ),
        outputs = [output],
        mnemonic = "Rustc",
    )
    return dep_crates
