use std::collections::VecDeque;

use crate::helper::safe_add;

use super::Instruction;

/// 評価時のエラー型
#[derive(Debug, PartialEq)]
pub enum EvalError {
    /// プログラムカウンタがオーバフロー
    PCOverFlow,
    /// スタックポインタがオーバフロー
    SPOverFlow,
    /// 不正なプログラムカウンタの入力
    InvalidPC,
    /// 不正なコンテキスト
    InvalidContext,
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EvaluationError: {self:?}")
    }
}

impl std::error::Error for EvalError {}

pub fn eval_depth(
    insts: &[Instruction],
    line: &[char],
    mut pc: usize,
    mut sp: usize,
) -> Result<bool, EvalError> {
    loop {
        let Some(next) = insts.get(pc) else {
            return Err(EvalError::InvalidPC);
        };
        match next {
            Instruction::Char(c) => {
                let Some(sp_c) = line.get(sp) else {
                    return Ok(false);
                };

                if c == sp_c {
                    safe_add(&mut pc, &1, || EvalError::PCOverFlow)?;
                    safe_add(&mut sp, &1, || EvalError::SPOverFlow)?;
                } else {
                    return Ok(false);
                }
            }
            Instruction::Any => {
                if line.get(sp).is_none() {
                    return Ok(false);
                };

                safe_add(&mut pc, &1, || EvalError::PCOverFlow)?;
                safe_add(&mut sp, &1, || EvalError::SPOverFlow)?;
            }
            Instruction::Start => {
                if sp != 0 {
                    return Ok(false);
                }
                safe_add(&mut pc, &1, || EvalError::PCOverFlow)?;
            }
            Instruction::End => {
                if sp != line.len() {
                    return Ok(false);
                }
                safe_add(&mut pc, &1, || EvalError::PCOverFlow)?;
            }
            Instruction::Match => {
                return Ok(true);
            }
            Instruction::Jump(addr) => {
                pc = *addr;
            }
            Instruction::Split(addr1, addr2) => {
                if eval_depth(insts, line, *addr1, sp)? || eval_depth(insts, line, *addr2, sp)? {
                    return Ok(true);
                } else {
                    return Ok(false);
                }
            }
        }
    }
}

fn eval_width(insts: &[Instruction], line: &[char]) -> Result<bool, EvalError> {
    let mut queue = VecDeque::<(usize, usize)>::new();
    let mut pc = 0;
    let mut sp = 0;
    loop {
        let Some(next) = insts.get(pc) else {
            return Err(EvalError::InvalidPC);
        };
        dbg!(next, pc, sp);
        match next {
            Instruction::Char(c) => {
                if let Some(sp_c) = line.get(sp) {
                    if sp_c == c {
                        safe_add(&mut pc, &1, || EvalError::PCOverFlow)?;
                        safe_add(&mut sp, &1, || EvalError::SPOverFlow)?;
                    } else {
                        // 分岐がもうないとき
                        if queue.is_empty() {
                            return Ok(false);
                        } else {
                            let Some(branch) = queue.pop_front() else {
                                return Err(EvalError::InvalidContext);
                            };
                            pc = branch.0;
                            sp = branch.1;
                        }
                    }
                } else if queue.is_empty() {
                    return Ok(false);
                };
            }
            Instruction::Any => {
                if line.get(sp).is_none() {
                    return Ok(false);
                }
                safe_add(&mut pc, &1, || EvalError::PCOverFlow)?;
                safe_add(&mut sp, &1, || EvalError::SPOverFlow)?;
            }
            Instruction::Start => {
                if sp == 0 {
                    safe_add(&mut pc, &1, || EvalError::PCOverFlow)?;
                } else if queue.is_empty() {
                    return Ok(false);
                } else {
                    let Some(branch) = queue.pop_front() else {
                        return Err(EvalError::InvalidContext);
                    };
                    pc = branch.0;
                    sp = branch.1;
                }
            }
            Instruction::End => {
                if sp == line.len() {
                    safe_add(&mut pc, &1, || EvalError::PCOverFlow)?;
                } else if queue.is_empty() {
                    return Ok(false);
                } else {
                    let Some(branch) = queue.pop_front() else {
                        return Err(EvalError::InvalidContext);
                    };
                    pc = branch.0;
                    sp = branch.1;
                }
            }
            Instruction::Match => {
                return Ok(true);
            }
            Instruction::Jump(addr) => {
                pc = *addr;
            }
            Instruction::Split(addr1, addr2) => {
                // プログラムカウンタをセットして、ブランチをプッシュ
                pc = *addr1;
                queue.push_back((*addr2, sp));
                continue;
            }
        }

        if !queue.is_empty() {
            queue.push_back((pc, sp));
            let Some(branch) = queue.pop_front() else {
                return Err(EvalError::InvalidContext);
            };
            pc = branch.0;
            sp = branch.1;
        }
    }
}

pub fn eval(insts: &[Instruction], line: &[char], is_depth: bool) -> Result<bool, EvalError> {
    if is_depth {
        eval_depth(insts, line, 0, 0)
    } else {
        eval_width(insts, line)
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::{codegen, parser};

    use super::*;

    fn to_insts(regex: &str) -> Vec<Instruction> {
        let ast = parser::parse(regex).unwrap();

        codegen::get_code(&ast).unwrap()
    }

    fn to_chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn test_simple() {
        let regex = "abc";
        let line = to_chars("abcde");
        let insts = to_insts(regex);

        let res = eval_depth(&insts, &line, 0, 0).unwrap();
        assert!(res);

        let res = eval_width(&insts, &line).unwrap();
        assert!(res)
    }

    #[test]
    fn test_question() {
        let regex = "a?";
        let line = to_chars("ab");
        let insts = to_insts(regex);

        let res = eval_depth(&insts, &line, 0, 0).unwrap();
        assert!(res);

        let res = eval_width(&insts, &line).unwrap();
        assert!(res)
    }

    #[test]
    fn test_plus() {
        let regex = "a+";
        let line = to_chars("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let insts = to_insts(regex);

        let res = eval_depth(&insts, &line, 0, 0).unwrap();
        assert!(res);

        let res = eval_width(&insts, &line).unwrap();
        assert!(res);

        let line = to_chars("b");
        let insts = to_insts(regex);

        let res = eval_depth(&insts, &line, 0, 0).unwrap();
        assert!(!res);

        let res = eval_width(&insts, &line).unwrap();
        assert!(!res)
    }

    #[test]
    fn test_star() {
        let regex = "a*";
        let line = to_chars("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let insts = to_insts(regex);

        let res = eval_depth(&insts, &line, 0, 0).unwrap();
        assert!(res);

        let res = eval_width(&insts, &line).unwrap();
        assert!(res);

        // `*`は0文字でもマッチするためこっちはなし
    }

    #[test]
    fn test_or() {
        let regex = "abc|123|def";
        let line = to_chars("def");
        let insts = to_insts(regex);

        let res = eval_depth(&insts, &line, 0, 0).unwrap();
        assert!(res);

        let res = eval_width(&insts, &line).unwrap();
        assert!(res);

        let line = to_chars("ab3");
        let insts = to_insts(regex);

        let res = eval_depth(&insts, &line, 0, 0).unwrap();
        assert!(!res);

        let res = eval_width(&insts, &line).unwrap();
        assert!(!res)
    }

    #[test]
    fn test_any() {
        let regex = "a.";
        let line = to_chars("ab");
        let insts = to_insts(regex);

        let res = eval_depth(&insts, &line, 0, 0).unwrap();
        assert!(res);

        let res = eval_width(&insts, &line).unwrap();
        assert!(res);

        let line = to_chars("a");
        let insts = to_insts(regex);

        let res = eval_depth(&insts, &line, 0, 0).unwrap();
        assert!(!res);

        let res = eval_width(&insts, &line).unwrap();
        assert!(!res)
    }

    #[test]
    fn test_start() {
        let regex = "^abc(^def|123)";
        let line = to_chars("abc123");
        let insts = to_insts(regex);

        let res = eval_depth(&insts, &line, 0, 0).unwrap();
        assert!(res);

        let res = eval_width(&insts, &line).unwrap();
        assert!(res);

        let line = to_chars("abcdef");

        let res = eval_depth(&insts, &line, 0, 0).unwrap();
        assert!(!res);

        let res = eval_width(&insts, &line).unwrap();
        assert!(!res)
    }

    #[test]
    fn test_end() {
        let regex = "abc(def|123$)+";
        let line = to_chars("abc123");
        let insts = to_insts(regex);

        let res = eval_depth(&insts, &line, 0, 0).unwrap();
        assert!(res);

        let res = eval_width(&insts, &line).unwrap();
        assert!(res);

        let line = to_chars("abc123def");

        let res = eval_depth(&insts, &line, 0, 0).unwrap();
        assert!(!res);

        let res = eval_width(&insts, &line).unwrap();
        assert!(!res)
    }
}
