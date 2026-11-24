pub fn add_two(x: u32) -> u32 {
    add_one::add_one(add_one::add_one(x))
}
