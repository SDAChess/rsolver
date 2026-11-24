def _run_test_impl(ctx):
    binary = ctx.executable.binary
    test = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = test, target_file = binary, is_executable = True)
    return [DefaultInfo(
        executable = test,
        runfiles = ctx.attr.binary[DefaultInfo].default_runfiles,
    )]

run_test = rule(
    implementation = _run_test_impl,
    attrs = {"binary": attr.label(mandatory = True, executable = True, cfg = "target")},
    test = True,
)

def _run_tests_impl(name, visibility, binaries, **kwargs):
    tests = []
    for binary in binaries:
        test = name + "_" + binary.name
        run_test(name = test, binary = binary, **kwargs)
        tests.append(test)
    native.test_suite(name = name, tests = tests, visibility = visibility)

run_tests = macro(
    doc = "One run_test per binary, named `<name>_<binary>`, plus a test_suite `name` over them.",
    implementation = _run_tests_impl,
    inherit_attrs = run_test,
    attrs = {
        "binaries": attr.label_list(mandatory = True, configurable = False),
        "binary": None,
    },
)
