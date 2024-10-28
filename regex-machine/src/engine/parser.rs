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

/// `|`をASTに変換する
fn fold_or(mut seq_or: Vec<AST>) -> Option<AST> {
    if seq_or.len() > 1 {
        let mut ast = seq_or.pop()?;
        seq_or.reverse();
        for s in seq_or {
            ast = AST::Or(Box::new(s), Box::new(ast))
        }
        Some(ast)
    } else {
        seq_or.pop()
    }
}

/// `parse`の内部状態を示す型
enum ParseState {
    /// 文字列処理中
    Char,
    /// エスケープ処理中
    Escape,
}

pub fn parse(expr: &str) -> Result<AST, ParseError> {
    let mut seq = Vec::new();
    let mut seq_or = Vec::new();
    // `()`が出てきたときに、それ以前の値を取っておく場所
    let mut stack = Vec::new();
    let mut state = ParseState::Char;

    for (idx, c) in expr.chars().enumerate() {
        match state {
            ParseState::Char => match c {
                '+' => parse_plus_star_question(&mut seq, PSQ::Plus, idx)?,
                '*' => parse_plus_star_question(&mut seq, PSQ::Star, idx)?,
                '?' => parse_plus_star_question(&mut seq, PSQ::Question, idx)?,
                '(' => {
                    // 現在の状態をスタックに避難させる
                    let prev = take(&mut seq);
                    let prev_or = take(&mut seq_or);
                    stack.push((prev, prev_or));
                }
                ')' => {
                    let Some((mut prev, prev_or)) = stack.pop() else {
                        return Err(ParseError::InvalidRightParen(idx));
                    };

                    // `(abc|def)`みたいなときに`def`が`seq`に入ってるので、`seq_or`に追加する
                    // `()`みたいなときは何もしない
                    if !seq.is_empty() {
                        seq_or.push(AST::Seq(seq));
                    }

                    if let Some(ast) = fold_or(seq_or) {
                        prev.push(ast);
                    }

                    // 過去の状態を復元する
                    seq = prev;
                    seq_or = prev_or;
                }
                '|' => {
                    if seq.is_empty() {
                        return Err(ParseError::NoPrev(idx));
                    } else {
                        let prev = take(&mut seq);
                        seq_or.push(AST::Seq(prev));
                    }
                }
                '\\' => state = ParseState::Escape,
                _ => {
                    seq.push(AST::Char(c));
                }
            },
            ParseState::Escape => {
                let ast = parse_escape(idx, c)?;
                seq.push(ast);
                state = ParseState::Char
            }
        };
    }

    // `)`が足りてないときはエラー
    // `(`と`)`が同じ数あるときは、スタックは空になるはず
    if !stack.is_empty() {
        return Err(ParseError::NoRightParen);
    };

    if !seq.is_empty() {
        seq_or.push(AST::Seq(seq));
    };

    if let Some(ast) = fold_or(seq_or) {
        Ok(ast)
    } else {
        Err(ParseError::Empty)
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

    #[test]
    fn valid_or() {
        // abc|123
        let seq = vec![
            AST::Seq(vec![AST::Char('a'), AST::Char('b'), AST::Char('c')]),
            AST::Seq(vec![AST::Char('1'), AST::Char('2'), AST::Char('3')]),
        ];

        let res = fold_or(seq).unwrap();

        assert_eq!(
            res,
            AST::Or(
                Box::new(AST::Seq(vec![
                    AST::Char('a'),
                    AST::Char('b'),
                    AST::Char('c')
                ])),
                Box::new(AST::Seq(vec![
                    AST::Char('1'),
                    AST::Char('2'),
                    AST::Char('3')
                ]))
            )
        );

        // foo
        let seq = vec![AST::Seq(vec![
            AST::Char('f'),
            AST::Char('o'),
            AST::Char('o'),
        ])];

        let res = fold_or(seq).unwrap();

        assert_eq!(
            res,
            AST::Seq(vec![AST::Char('f'), AST::Char('o'), AST::Char('o'),])
        )
    }

    #[test]
    #[should_panic]
    fn invalid_or() {
        // empty
        let seq = vec![];

        fold_or(seq).unwrap();
    }

    #[test]
    fn simple_regex() {
        let regex = "abc";

        let ast = parse(regex).unwrap();

        assert_eq!(
            ast,
            AST::Seq(vec![AST::Char('a'), AST::Char('b'), AST::Char('c'),])
        )
    }

    #[test]
    fn escaped_regex() {
        let regex = r"1\?\*23";

        let ast = parse(regex).unwrap();

        assert_eq!(
            ast,
            AST::Seq(vec![
                AST::Char('1'),
                AST::Char('?'),
                AST::Char('*'),
                AST::Char('2'),
                AST::Char('3')
            ])
        )
    }

    #[test]
    fn plus_star_question_regex() {
        let regex = r"b?+*";

        let ast = parse(regex).unwrap();

        assert_eq!(
            ast,
            AST::Seq(vec![AST::Star(Box::new(AST::Plus(Box::new(
                AST::Question(Box::new(AST::Char('b')))
            ))))])
        )
    }

    #[test]
    fn or_regex() {
        let regex = r"abc|123";

        let ast = parse(regex).unwrap();

        assert_eq!(
            ast,
            AST::Or(
                Box::new(AST::Seq(vec![
                    AST::Char('a'),
                    AST::Char('b'),
                    AST::Char('c'),
                ])),
                Box::new(AST::Seq(vec![
                    AST::Char('1'),
                    AST::Char('2'),
                    AST::Char('3'),
                ]))
            )
        )
    }

    #[test]
    fn nested_paren_regex() {
        let regex = r"(abc(123)def)";

        let ast = parse(regex).unwrap();

        assert_eq!(
            ast,
            AST::Seq(vec![AST::Seq(vec![
                AST::Char('a'),
                AST::Char('b'),
                AST::Char('c'),
                AST::Seq(vec![AST::Char('1'), AST::Char('2'), AST::Char('3'),]),
                AST::Char('d'),
                AST::Char('e'),
                AST::Char('f')
            ]),])
        )
    }

    #[test]
    fn invalid_right_paren() {
        let regex = r"abc)";

        let err = parse(regex).err().unwrap();
        assert_eq!(err, ParseError::InvalidRightParen(3))
    }

    #[test]
    fn missing_right_paren() {
        let regex = r"(abc(123)";

        let err = parse(regex).err().unwrap();
        assert_eq!(err, ParseError::NoRightParen)
    }
}
