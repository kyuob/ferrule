/// A small bridge smoke example. Put application rules in domain modules.
#[flutter_rust_bridge::frb(sync)]
pub fn add_values(left: i32, right: i32) -> i64 {
    crate::usecase::arithmetic::add_values(left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition_does_not_overflow_i32() {
        assert_eq!(add_values(i32::MAX, 1), 2_147_483_648);
    }
}
