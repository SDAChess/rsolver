RustCrateInfo = provider(
    doc = "A compiled Rust crate.",
    fields = {
        "rlib": "File: the compiled rlib.",
        "crate_name": "string: the name dependents pass to --extern.",
        "transitive_rlibs": "depset of Files: this crate's rlib and those of all its deps.",
    },
)
