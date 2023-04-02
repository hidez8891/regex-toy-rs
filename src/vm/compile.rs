use crate::parser::{
    ast::{Ast, AstKind, GreedyKind, MatchKind, PositionKind, RepeatKind},
    parser::Parser,
};

use super::inst::Inst;

pub struct Compiler {}

impl Compiler {
    pub fn compile(pattern: &str) -> Result<Vec<Inst>, String> {
        let ast = Parser::parse(pattern)?;
        let insts = Self::compile_from(&ast);
        Ok(insts)
    }

    pub fn compile_from(ast: &Ast) -> Vec<Inst> {
        let mut insts = Self::compile_root(ast);
        insts.push(Inst::Success);
        insts
    }

    fn compile_root(ast: &Ast) -> Vec<Inst> {
        match &ast.kind {
            AstKind::Group => Self::compile_group(ast),
            AstKind::Union => Self::compile_union(ast),
            AstKind::IncludeSet => Self::compile_include_set(ast),
            AstKind::ExcludeSet => Self::compile_exclude_set(ast),
            AstKind::Star(greedy) => Self::compile_star(ast, greedy),
            AstKind::Plus(greedy) => Self::compile_plus(ast, greedy),
            AstKind::Option(greedy) => Self::compile_option(ast, greedy),
            AstKind::Repeat(n, m, greedy) => match (n, m) {
                (RepeatKind::Num(n), RepeatKind::Num(m)) => {
                    if n == m {
                        Self::compile_repeat_count(ast, *n)
                    } else {
                        Self::compile_repeat_range(ast, *n, *m, greedy)
                    }
                }
                (RepeatKind::Num(c), RepeatKind::Infinity) => {
                    Self::compile_repeat_min(ast, *c, greedy)
                }
                (RepeatKind::Infinity, _) => {
                    unreachable!()
                }
            },
            AstKind::Match(kind) => Self::compile_match(ast, kind),
            AstKind::Position(kind) => Self::compile_position(ast, kind),
            AstKind::None => unreachable!(),
        }
    }

    fn compile_group(ast: &Ast) -> Vec<Inst> {
        let mut insts = Vec::new();
        for child in ast.children.iter() {
            insts.extend(Self::compile_root(child));
        }
        insts
    }

    fn compile_union(ast: &Ast) -> Vec<Inst> {
        let mut insts = Vec::new();

        let mut dst_addr = 2;
        insts.push(Inst::Fail);

        for child in ast.children.iter().rev() {
            let mut child_insts = Self::compile_root(child);
            child_insts.reverse();

            let next_addr = child_insts.len() as isize + 2;

            insts.push(Inst::Jmp(dst_addr));
            insts.extend(child_insts);
            insts.push(Inst::Split(1, next_addr));

            dst_addr += next_addr;
        }

        insts.reverse();
        insts
    }

    fn compile_include_set(ast: &Ast) -> Vec<Inst> {
        let mut insts = Self::compile_include_set_impl(ast, 2);
        insts.reverse();
        insts.push(Inst::Fail);
        insts.push(Inst::ConsumeRead);
        insts
    }

    fn compile_include_set_impl(ast: &Ast, dst_addr: isize) -> Vec<Inst> {
        let mut insts = Vec::new();
        let mut dst_addr = dst_addr;
        for child in ast.children.iter().rev() {
            match &child.kind {
                AstKind::Group => {
                    let item = Self::compile_include_set_impl(child, dst_addr);
                    dst_addr += item.len() as isize;

                    insts.extend(item);
                }
                AstKind::Match(kind) => match kind {
                    MatchKind::Char(c) => {
                        insts.push(Inst::JmpIfInclude(*c, *c, dst_addr));
                        dst_addr += 1;
                    }
                    MatchKind::Range(a, b) => {
                        insts.push(Inst::JmpIfInclude(*a, *b, dst_addr));
                        dst_addr += 1;
                    }
                    MatchKind::Any => unreachable!(),
                },
                _ => unreachable!(),
            }
        }
        insts
    }

    fn compile_exclude_set(ast: &Ast) -> Vec<Inst> {
        let mut insts = Vec::new();
        for child in ast.children.iter() {
            match &child.kind {
                AstKind::Group => {
                    let item = Self::compile_exclude_set(child);
                    insts.extend(item);
                }
                AstKind::Match(kind) => match kind {
                    MatchKind::Char(c) => {
                        insts.push(Inst::SkipReadIfExclude(*c, *c));
                    }
                    MatchKind::Range(a, b) => {
                        insts.push(Inst::SkipReadIfExclude(*a, *b));
                    }
                    MatchKind::Any => unreachable!(),
                },
                _ => unreachable!(),
            }
        }
        insts.push(Inst::ConsumeRead);
        insts
    }

    fn compile_star(ast: &Ast, greedy: &GreedyKind) -> Vec<Inst> {
        let child_insts = Self::compile_root(&ast.children[0]);
        let child_size = child_insts.len() as isize;

        let mut insts = Vec::new();
        if matches!(greedy, &GreedyKind::Greedy) {
            insts.push(Inst::Split(1, child_size + 2));
        } else {
            insts.push(Inst::Split(child_size + 2, 1));
        }
        insts.extend(child_insts);
        insts.push(Inst::Jmp(-child_size - 1));

        insts
    }

    fn compile_plus(ast: &Ast, greedy: &GreedyKind) -> Vec<Inst> {
        let child_insts = Self::compile_root(&ast.children[0]);
        let child_size = child_insts.len() as isize;

        let mut insts = Vec::new();
        insts.extend(child_insts);
        if matches!(greedy, &GreedyKind::Greedy) {
            insts.push(Inst::Split(-child_size, 1));
        } else {
            insts.push(Inst::Split(1, -child_size));
        }

        insts
    }

    fn compile_option(ast: &Ast, greedy: &GreedyKind) -> Vec<Inst> {
        let child_insts = Self::compile_root(&ast.children[0]);
        let child_size = child_insts.len() as isize;

        let mut insts = Vec::new();
        if matches!(greedy, &GreedyKind::Greedy) {
            insts.push(Inst::Split(1, child_size + 1));
        } else {
            insts.push(Inst::Split(child_size + 1, 1));
        }
        insts.extend(child_insts);

        insts
    }

    fn compile_repeat_count(ast: &Ast, count: u32) -> Vec<Inst> {
        let child_insts = Self::compile_root(&ast.children[0]);

        let mut insts = Vec::new();
        for _ in 0..count {
            insts.extend(child_insts.clone());
        }
        insts
    }

    fn compile_repeat_min(ast: &Ast, count: u32, greedy: &GreedyKind) -> Vec<Inst> {
        let mut insts = Vec::new();
        insts.extend(Self::compile_repeat_count(ast, count));
        insts.extend(Self::compile_star(ast, greedy));
        insts
    }

    fn compile_repeat_range(ast: &Ast, min: u32, max: u32, greedy: &GreedyKind) -> Vec<Inst> {
        let mut child_insts = Self::compile_root(&ast.children[0]);
        child_insts.reverse();

        let mut insts = Vec::new();
        let mut dst_addr = 1;
        for _ in min..max {
            dst_addr += child_insts.len() as isize;

            insts.extend(child_insts.clone());
            if matches!(greedy, &GreedyKind::Greedy) {
                insts.push(Inst::Split(1, dst_addr));
            } else {
                insts.push(Inst::Split(dst_addr, 1));
            }
        }

        let mut repeat_insts = Self::compile_repeat_count(ast, min);
        repeat_insts.reverse();
        insts.extend(repeat_insts);

        insts.reverse();
        insts
    }

    fn compile_match(ast: &Ast, kind: &MatchKind) -> Vec<Inst> {
        match kind {
            MatchKind::Any => [Inst::Any].into(),
            MatchKind::Char(c) => [Inst::Char(*c)].into(),
            MatchKind::Range(a, b) => unreachable!(),
        }
    }

    fn compile_position(ast: &Ast, position: &PositionKind) -> Vec<Inst> {
        match position {
            PositionKind::SoL => [Inst::PosSOL].into(),
            PositionKind::EoL => [Inst::PosEOL].into(),
        }
    }
}
