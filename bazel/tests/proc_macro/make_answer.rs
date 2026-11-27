use proc_macro::TokenStream;

/// `make_answer!(name)` expands to `pub fn name() -> u32 { 42 }`.
#[proc_macro]
pub fn make_answer(name: TokenStream) -> TokenStream {
    let value = answer::value();
    format!("pub fn {name}() -> u32 {{ {value} }}")
        .parse()
        .unwrap()
}
