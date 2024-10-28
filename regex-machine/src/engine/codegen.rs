use super::{parser::AST, Instruction};
use crate::helper::safe_add;

#[derive(Debug)]
pub enum CodeGenError {
    /// プログラムカウンタがオーバフロー
    PCOverFlow,
    FailStar,
    FailOr,
    FailQuestion,
}

impl std::fmt::Display for CodeGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CodeGenError: {self:?}")
    }
}

impl std::error::Error for CodeGenError {}

#[derive(Debug, Default)]
pub struct Generator {
    pc: usize,
    insts: Vec<Instruction>,
}

impl Generator {
    /// プログラムカウンタをインクリメント
    fn inc_pc(&mut self) -> Result<(), CodeGenError> {
        safe_add(&mut self.pc, &1, || CodeGenError::PCOverFlow)
    }

    fn gen_expr(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        match ast {
            AST::Char(c) => self.gen_char(c),
            AST::Plus(ast) => self.gen_plus(ast),
            AST::Star(ast) => self.gen_star(ast),
            AST::Question(ast) => self.gen_question(ast),
            AST::Or(e1, e2) => self.gen_or(e1, e2),
            AST::Seq(seq) => self.gen_seq(seq),
        }
    }

    fn gen_char(&mut self, c: &char) -> Result<(), CodeGenError> {
        let inst = Instruction::Char(*c);
        self.insts.push(inst);
        self.inc_pc()?;
        Ok(())
    }

    fn gen_seq(&mut self, exprs: &[AST]) -> Result<(), CodeGenError> {
        for e in exprs {
            self.gen_expr(e)?
        }
        Ok(())
    }

    fn gen_plus(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        let start_addr = self.pc;
        self.gen_expr(ast)?;

        self.inc_pc()?;
        let split = Instruction::Split(start_addr, self.pc);
        self.insts.push(split);

        Ok(())
    }

    fn gen_star(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        let split_addr = self.pc;
        self.inc_pc()?;

        let split = Instruction::Split(self.pc, 0);
        self.insts.push(split);

        self.gen_expr(ast)?;

        // はじめの`split`へ戻る
        let jump = Instruction::Jump(split_addr);
        self.insts.push(jump);
        self.inc_pc()?;

        if let Some(Instruction::Split(_, l2)) = self.insts.get_mut(split_addr) {
            *l2 = self.pc;
        } else {
            return Err(CodeGenError::FailStar);
        }

        Ok(())
    }

    fn gen_question(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        let split_addr = self.pc;
        self.inc_pc()?;
        // 次の行に飛ぶか、その終わりに飛ぶか。`ast`の次の行は`ast`を生成しないと値が分からないので、仮に0を設定しておく
        let split = Instruction::Split(self.pc, 0);
        self.insts.push(split);

        self.gen_expr(ast)?;

        if let Some(Instruction::Split(_, l2)) = self.insts.get_mut(split_addr) {
            *l2 = self.pc;
        } else {
            return Err(CodeGenError::FailQuestion);
        }

        Ok(())
    }

    fn gen_or(&mut self, e1: &AST, e2: &AST) -> Result<(), CodeGenError> {
        // `split`がある行
        let split_addr = self.pc;
        self.inc_pc()?;

        // `e2`は`e1`を生成しないと値が分からないので、仮に0を設定しておく
        let split = Instruction::Split(self.pc, 0);

        self.insts.push(split);
        self.gen_expr(e1)?;

        let jmp_addr = self.pc;
        // 本当は`e2`の次の値を入れたいが、生成しないとわからないので仮に0を設定しておく
        self.insts.push(Instruction::Jump(0));

        self.inc_pc()?;
        // `e2`の始まる位置が確定したので、`split`を正しいものにする
        if let Some(Instruction::Split(_, l2)) = self.insts.get_mut(split_addr) {
            *l2 = self.pc;
        } else {
            return Err(CodeGenError::FailOr);
        }

        self.gen_expr(e2)?;

        if let Some(Instruction::Jump(l3)) = self.insts.get_mut(jmp_addr) {
            *l3 = self.pc;
        } else {
            return Err(CodeGenError::FailOr);
        }

        Ok(())
    }

    fn gen_code(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        self.gen_expr(ast)?;
        self.inc_pc()?;
        self.insts.push(Instruction::Match);
        Ok(())
    }
}

pub fn get_code(ast: &AST) -> Result<Vec<Instruction>, CodeGenError> {
    let mut generator = Generator::default();
    generator.gen_code(ast)?;
    Ok(generator.insts)
}

#[cfg(test)]
mod tests {
    use crate::engine::parser;

    use super::*;

    #[test]
    fn char_regex() {
        let regex_str = "a";
        let ast = parser::parse(regex_str).unwrap();

        let mut generator = Generator::default();

        generator.gen_expr(&ast).unwrap();

        let expected = vec![Instruction::Char('a')];

        assert_eq!(generator.insts, expected)
    }

    #[test]
    fn seq_regex() {
        let regex_str = "foobar";
        let ast = parser::parse(regex_str).unwrap();

        let mut generator = Generator::default();

        generator.gen_expr(&ast).unwrap();

        let expected = vec![
            Instruction::Char('f'),
            Instruction::Char('o'),
            Instruction::Char('o'),
            Instruction::Char('b'),
            Instruction::Char('a'),
            Instruction::Char('r'),
        ];

        assert_eq!(generator.insts, expected)
    }

    #[test]
    fn plus_regex() {
        let regex_str = "a+";
        let ast = parser::parse(regex_str).unwrap();

        let mut generator = Generator::default();

        generator.gen_expr(&ast).unwrap();

        let expected = vec![Instruction::Char('a'), Instruction::Split(0, 2)];

        assert_eq!(generator.insts, expected)
    }

    #[test]
    fn star_regex() {
        let regex_str = "a*";
        let ast = parser::parse(regex_str).unwrap();

        let mut generator = Generator::default();

        generator.gen_expr(&ast).unwrap();

        let expected = vec![
            Instruction::Split(1, 3),
            Instruction::Char('a'),
            Instruction::Jump(0),
        ];

        assert_eq!(generator.insts, expected)
    }

    #[test]
    fn question_regex() {
        let regex_str = "a?";
        let ast = parser::parse(regex_str).unwrap();

        let mut generator = Generator::default();

        generator.gen_expr(&ast).unwrap();

        let expected = vec![Instruction::Split(1, 2), Instruction::Char('a')];

        assert_eq!(generator.insts, expected)
    }

    #[test]
    fn or_regex() {
        let regex_str = "abc|123";
        let ast = parser::parse(regex_str).unwrap();

        let mut generator = Generator::default();

        generator.gen_expr(&ast).unwrap();

        let expected = vec![
            Instruction::Split(1, 5),
            Instruction::Char('a'),
            Instruction::Char('b'),
            Instruction::Char('c'),
            Instruction::Jump(8),
            Instruction::Char('1'),
            Instruction::Char('2'),
            Instruction::Char('3'),
        ];

        assert_eq!(generator.insts, expected)
    }
}
