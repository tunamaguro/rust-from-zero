//! 正規表現用エンジン
//!
//! ```
//! use regex_machine::{do_matching,print};
//! let expr = "abc|(de|cd)+";
//! let line = "decddede";
//! assert!(do_matching(expr,line,true).unwrap());
//! assert!(print(expr).is_ok());
//! ```

pub mod engine;
mod helper;

pub use engine::{do_matching, print};
