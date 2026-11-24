RustCrateInfo = provider(
    doc = "A compiled Rust crate.",
    fields = {
        "rlib": "File: the compiled rlib.",
        "crate_name": "string: the name dependents pass to --extern.",
    },
)
