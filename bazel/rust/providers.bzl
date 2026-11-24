RustCrateInfo = provider(
    doc = "A compiled Rust crate.",
    fields = {
        "rlib": "File: the compiled rlib.",
        "crate_name": "string: the name dependents pass to --extern.",
        "transitive_crates": "depset of Files: this crate's rlib, its deps' rlibs and every proc-macro .so they use.",
    },
)

RustProcMacroInfo = provider(
    doc = "A compiled proc-macro crate. Its deps are deliberately absent: consumers never need them.",
    fields = {
        "so": "File: the compiled proc-macro, a shared library rustc loads.",
        "crate_name": "string: the name dependents pass to --extern.",
    },
)
