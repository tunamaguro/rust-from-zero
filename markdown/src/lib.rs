//! # これはファイルに対するドキュメンテーションコメントです
//!
//! ## 第2見出し
//!
//! - aaaa
//! - bbbb
//!
//! ```
//! println!("Document");
//! ```
//!

pub mod foo {
    //! モジュールコメント
    //!
    //! # これはモジュールに対するドキュメントです
}

/// 足し算
///
/// ## Example
///
/// ```
/// use markdown::add;
/// assert_eq!(5,add(3,2));
/// ```
///
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    #[should_panic]
    fn it_panic() {
        panic!("this should panic");
    }
}
