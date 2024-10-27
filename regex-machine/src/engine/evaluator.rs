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

fn eval_depth(
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
