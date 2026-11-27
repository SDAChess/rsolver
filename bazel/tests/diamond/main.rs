fn main() {
    // 1 + 2: both `util` crates must be linked, each under its own identity.
    assert_eq!(top::sum(), 3);
}
