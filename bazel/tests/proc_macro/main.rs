fn main() {
    // `from_lib` only exists if the macro expanded inside `uses_macro`.
    assert_eq!(uses_macro::from_lib(), 42);
}
