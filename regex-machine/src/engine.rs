use crate::helper::DynError;

mod codegen;
mod evaluator;
mod parser;

/// 内部的に扱う疑似アセンブリの型  
/// P131を参照のこと
#[derive(Debug, PartialEq)]
pub enum Instruction {
    /// 入力を1文字使って、`char`と等しいか検証する
    Char(char),
    /// マッチ成功
    Match,
    /// `usize`までジャンプ
    Jump(usize),
    /// それぞれを検証
    Split(usize, usize),
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Char(c) => write!(f, "char {c}"),
            Instruction::Match => write!(f, "match"),
            Instruction::Jump(x) => write!(f, "jmp {x:>04}"),
            Instruction::Split(x, y) => write!(f, "split {x:>04}, {y:>04}"),
        }
    }
}

/// 正規表現を用いて、文字列とマッチングを行う
///
/// ```
/// use regex_machine::engine;
/// assert!(engine::do_matching("abc|(de|cd)+","decddede",true).unwrap());
/// ```
/// 
pub fn do_matching(expr: &str, line: &str, is_depth: bool) -> Result<bool, DynError> {
    let ast = parser::parse(expr).map_err(Box::new)?;
    let code = codegen::get_code(&ast).map(Box::new)?;
    let line = line.chars().collect::<Vec<char>>();
    let result = evaluator::eval(&code, &line, is_depth).map_err(Box::new)?;

    Ok(result)
}
