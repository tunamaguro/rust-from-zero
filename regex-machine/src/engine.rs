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
