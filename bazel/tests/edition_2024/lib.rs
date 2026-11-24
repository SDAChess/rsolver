// Let chains only compile in edition 2024.
pub fn is_positive(value: Option<i32>) -> bool {
    if let Some(x) = value
        && x > 0
    {
        return true;
    }
    false
}
