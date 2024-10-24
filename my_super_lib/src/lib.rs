/// 偶数の乱数を返す
///
/// ```
/// use my_super_lib::rand_even;
/// assert_eq!(rand_even() % 2, 0);
/// ```
pub fn rand_even() -> u32 {
    rand::random::<u32>() & !1
}
