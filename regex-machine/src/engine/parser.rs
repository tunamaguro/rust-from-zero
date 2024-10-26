use std::{
    error::Error,
    fmt::{self, Display},
    mem::take,
};

/// 正規表現のAST
#[derive(Debug, PartialEq)]
pub enum AST {
    /// 1文字
    Char(char),
    /// 1回以上の繰り返し
    Plus(Box<AST>),
    /// 0回以上の繰り返し
    Star(Box<AST>),
    /// 高々1回の繰り返し
    Question(Box<AST>),
    /// どっちか
    Or(Box<AST>, Box<AST>),
    /// 複数の正規表現をまとめたもの
    Seq(Vec<AST>),
}

/// 正規表現をパースする際のエラー
#[derive(Debug, PartialEq)]
pub enum ParseError {
    /// 誤ったエスケープシーケンス
    InvalidEscape(usize, char),
    /// 開き括弧`(`なし
    InvalidRightParen(usize),
    /// `+`,`?`,`*`,`|`の前に正規表現がない
    NoPrev(usize),
    /// 閉じ括弧`)`がない
    NoRightParen,
    /// 空っぽ
    Empty,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidEscape(pos, c) => {
                write!(f, "ParseError: invalid escape: pos = {}, char = {}", pos, c)
            }
            ParseError::InvalidRightParen(pos) => {
                write!(f, "ParseError: invalid right parenthesis: pos = {}", pos)
            }
            ParseError::NoPrev(pos) => {
                write!(f, "ParseError: no previous expression: pos = {}", pos)
            }
            ParseError::NoRightParen => {
                write!(f, "ParseError: no right parenthesis")
            }
            ParseError::Empty => {
                write!(f, "ParseError: empty expression")
            }
        }
    }
}

// ParseErrorが`Debug`と`Display`を実装しているため自動で実装される
impl Error for ParseError {}

/// 特殊文字のエスケープ
fn parse_escape(pos: usize, c: char) -> Result<AST, ParseError> {
    match c {
        '\\' | '(' | ')' | '|' | '+' | '*' | '?' => Ok(AST::Char(c)),
        _ => {
            let err = ParseError::InvalidEscape(pos, c);
            Err(err)
        }
    }
}

enum PSQ {
    Plus,
    Star,
    Question,
}

/// `+`.`*`,`?`をASTに変換する
///
/// その前にパターンがない場合はエラー
fn parse_plus_star_question(
    seq: &mut Vec<AST>,
    ast_type: PSQ,
    pos: usize,
) -> Result<(), ParseError> {
    // １つ前のパターンを使うので、1つ最後尾から取り出す
    if let Some(prev) = seq.pop() {
        let prev_box = Box::new(prev);
        let ast = match ast_type {
            PSQ::Plus => AST::Plus(prev_box),
            PSQ::Star => AST::Star(prev_box),
            PSQ::Question => AST::Question(prev_box),
        };

        seq.push(ast);
        Ok(())
    } else {
        Err(ParseError::NoPrev(pos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_parse_escape() {
        assert_eq!(parse_escape(3, '+').unwrap(), AST::Char('+'));
        assert_eq!(parse_escape(1, '|').unwrap(), AST::Char('|'))
    }

    #[test]
    fn invalid_parse_escape() {
        assert_eq!(
            parse_escape(3, 'a').err().unwrap(),
            ParseError::InvalidEscape(3, 'a')
        );
        assert_eq!(
            parse_escape(123, 'b').err().unwrap(),
            ParseError::InvalidEscape(123, 'b')
        )
    }

    #[test]
    fn valid_plus_star_question() {
        let mut seq = vec![AST::Char('6')];
        parse_plus_star_question(&mut seq, PSQ::Plus, 1).unwrap();
        assert_eq!(*seq.last().unwrap(), AST::Plus(Box::new(AST::Char('6'))));

        let mut seq = vec![AST::Char('j')];
        parse_plus_star_question(&mut seq, PSQ::Question, 1).unwrap();
        assert_eq!(
            *seq.last().unwrap(),
            AST::Question(Box::new(AST::Char('j')))
        );

        let mut seq = vec![AST::Char('u')];
        parse_plus_star_question(&mut seq, PSQ::Star, 1).unwrap();
        assert_eq!(*seq.last().unwrap(), AST::Star(Box::new(AST::Char('u'))));
    }

    #[test]
    fn invalid_plus_star_question() {
        let mut seq = vec![];
        assert_eq!(
            parse_plus_star_question(&mut seq, PSQ::Plus, 1)
                .err()
                .unwrap(),
            ParseError::NoPrev(1)
        );
    }
}
