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

/// 正規表現をパースした結果を標準出力に出す
///
/// ```
/// use regex_machine::print;
/// assert!(print("abc|(de|cd)+").is_ok());
/// ```
///
/// ## 返値
/// 与えられた正規表現にエラーがある場合、`Err`を返す。そうでない場合、出力は標準出力に出るため返値はない
///
pub fn print(expr: &str) -> Result<(), DynError> {
    let ast = parser::parse(expr)?;

    println!("Ast: {ast:?}");

    let code = codegen::get_code(&ast).map_err(Box::new)?;
    let code = code.iter().map(|inst| inst.to_string()).collect::<Vec<_>>();
    println!("code:");
    println!("{}", code.join("\n"));

    Ok(())
}

/// 正規表現を用いて、文字列とマッチングを行う
///
/// ```
/// use regex_machine::do_matching;
/// assert!(do_matching("abc|(de|cd)+","decddede",true).unwrap());
/// ```
///
/// ## 引数
/// - `expr`: 評価に用いる正規表現。`(`のような文字はエスケープ処理が必要(例: `\(`)
/// - `line`: `expr`にマッチするかどうか検証する文字列
/// - `is_depth`: `true`のとき深さ優先探索をする。`false`の時は幅優先探索をする
///
/// ## 返値
/// エラーなく実行でき、かつマッチした場合は`Ok(true)`を返す。エラーなく実行でき、マッチしなかった場合は`Ok(false)`を返す
///
pub fn do_matching(expr: &str, line: &str, is_depth: bool) -> Result<bool, DynError> {
    let ast = parser::parse(expr)?;
    let code = codegen::get_code(&ast)?;
    let line = line.chars().collect::<Vec<char>>();
    let result = evaluator::eval(&code, &line, is_depth)?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_do_matching() {
        // パースエラー
        assert!(do_matching("+b", "bbb", true).is_err());
        assert!(do_matching("*b", "bbb", true).is_err());
        assert!(do_matching("|b", "bbb", true).is_err());
        assert!(do_matching("?b", "bbb", true).is_err());

        // パース成功、マッチ成功
        assert!(do_matching("abc|def", "def", true).unwrap());
        assert!(do_matching("(abc)*", "abcabc", true).unwrap());
        assert!(do_matching("(ab|cd)+", "abcd", true).unwrap());
        assert!(do_matching("abc?", "abcd", true).unwrap());

        // パース成功、マッチ失敗
        assert!(!do_matching("abc|def", "abd", true).unwrap());
        assert!(!do_matching("(ab|cd)+", "", true).unwrap());
        assert!(!do_matching("abc?", "acd", true).unwrap());
    }
}
