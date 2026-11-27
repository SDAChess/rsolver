def _build_test_impl(ctx):
    script = ctx.actions.declare_file(ctx.label.name + ".sh")
    ctx.actions.write(script, "#!/bin/sh\nexit 0\n", is_executable = True)

    # Putting the targets' outputs in runfiles is what makes `bazel test` build them.
    files = depset(transitive = [t[DefaultInfo].files for t in ctx.attr.targets])
    return [DefaultInfo(
        executable = script,
        runfiles = ctx.runfiles(transitive_files = files),
    )]

build_test = rule(
    implementation = _build_test_impl,
    attrs = {"targets": attr.label_list(mandatory = True)},
    test = True,
)
