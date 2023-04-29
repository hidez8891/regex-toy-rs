use super::inst::Inst;
use crate::parser::{
    ast::{AstKind, GreedyKind, MatchKind, PositionKind, RepeatKind},
    Ast,
};

pub(crate) struct Compiler {
    max_capture_id: usize,
}

impl Compiler {
    pub fn compile(ast: &Ast) -> (Vec<Inst>, usize) {
        let mut compiler = Compiler { max_capture_id: 0 };

        let mut insts = compiler.compile_root(ast);
        insts.insert(0, Inst::CaptureStart(0));
        insts.push(Inst::CaptureEnd(0));
        insts.push(Inst::Success);

        (insts, compiler.max_capture_id + 1)
    }

    fn compile_root(&mut self, ast: &Ast) -> Vec<Inst> {
        match &ast.kind {
            AstKind::CaptureGroup(cap_id) => {
                if self.max_capture_id < *cap_id {
                    self.max_capture_id = *cap_id;
                }
                self.compile_group(ast, *cap_id)
            }
            AstKind::NonCaptureGroup => self.compile_group(ast, 0),
            AstKind::Union => self.compile_union(ast),
            AstKind::IncludeSet => self.compile_include_set(ast),
            AstKind::ExcludeSet => self.compile_exclude_set(ast),
            AstKind::Star(greedy) => self.compile_star(ast, greedy),
            AstKind::Plus(greedy) => self.compile_plus(ast, greedy),
            AstKind::Option(greedy) => self.compile_option(ast, greedy),
            AstKind::Repeat(n, m, greedy) => self.compile_repeat(ast, n, m, greedy),
            AstKind::Match(kind) => Self::compile_match(kind),
            AstKind::Position(kind) => Self::compile_position(kind),
        }
    }

    fn compile_group(&mut self, ast: &Ast, cap_id: usize) -> Vec<Inst> {
        let mut insts = Vec::new();

        if cap_id > 0 {
            insts.push(Inst::CaptureStart(cap_id))
        }
        for child in ast.children.iter() {
            insts.extend(self.compile_root(child));
        }
        if cap_id > 0 {
            insts.push(Inst::CaptureEnd(cap_id))
        }

        insts
    }

    fn compile_union(&mut self, ast: &Ast) -> Vec<Inst> {
        let mut insts = Vec::new();

        let mut dst_addr = 2;
        insts.push(Inst::Fail);

        for child in ast.children.iter().rev() {
            let mut child_insts = self.compile_root(child);
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

    fn compile_include_set(&mut self, ast: &Ast) -> Vec<Inst> {
        let mut insts = Vec::new();
        insts.push(Inst::Seek(1));
        insts.push(Inst::Fail);

        let mut dst_addr = 2;

        for child in ast.children.iter().rev() {
            match &child.kind {
                AstKind::Match(kind) => match kind {
                    MatchKind::Char(c) => {
                        insts.push(Inst::JmpIfTrue(dst_addr));
                        insts.push(Inst::CheckInclude(*c, *c));
                        dst_addr += 2;
                    }
                    MatchKind::Range(a, b) => {
                        insts.push(Inst::JmpIfTrue(dst_addr));
                        insts.push(Inst::CheckInclude(*a, *b));
                        dst_addr += 2;
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
        }

        insts.reverse();
        insts
    }

    fn compile_exclude_set(&mut self, ast: &Ast) -> Vec<Inst> {
        let mut insts = Vec::new();
        insts.push(Inst::Fail);
        insts.push(Inst::Jmp(2));
        insts.push(Inst::Seek(1));

        let mut fail_addr = 3;

        for child in ast.children.iter().rev() {
            match &child.kind {
                AstKind::Match(kind) => match kind {
                    MatchKind::Char(c) => {
                        insts.push(Inst::JmpIfFalse(fail_addr));
                        insts.push(Inst::CheckExclude(*c, *c));
                        fail_addr += 2;
                    }
                    MatchKind::Range(a, b) => {
                        insts.push(Inst::JmpIfFalse(fail_addr));
                        insts.push(Inst::CheckExclude(*a, *b));
                        fail_addr += 2;
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
        }

        insts.reverse();
        insts
    }

    fn compile_star(&mut self, ast: &Ast, greedy: &GreedyKind) -> Vec<Inst> {
        let child_insts = self.compile_root(&ast.children[0]);
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

    fn compile_plus(&mut self, ast: &Ast, greedy: &GreedyKind) -> Vec<Inst> {
        let child_insts = self.compile_root(&ast.children[0]);
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

    fn compile_option(&mut self, ast: &Ast, greedy: &GreedyKind) -> Vec<Inst> {
        let child_insts = self.compile_root(&ast.children[0]);
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

    #[rustfmt::skip]
    fn compile_repeat(&mut self,
        ast: &Ast,
        min: &RepeatKind,
        max: &RepeatKind,
        greedy: &GreedyKind,
    ) -> Vec<Inst> {
        match (min, max) {
            (RepeatKind::Num(n), RepeatKind::Num(m)) if n == m => {
                self.compile_repeat_count(ast, *n)
            }
            (RepeatKind::Num(n), RepeatKind::Num(m)) => {
                self.compile_repeat_range(ast, *n, *m, greedy)
            }
            (RepeatKind::Num(c), RepeatKind::Infinity) => {
                self.compile_repeat_min(ast, *c, greedy)
            }
            (RepeatKind::Infinity, _) => {
                unreachable!()
            }
        }
    }

    fn compile_repeat_count(&mut self, ast: &Ast, count: u32) -> Vec<Inst> {
        let child_insts = self.compile_root(&ast.children[0]);

        let mut insts = Vec::new();
        for _ in 0..count {
            insts.extend(child_insts.clone());
        }
        insts
    }

    fn compile_repeat_min(&mut self, ast: &Ast, count: u32, greedy: &GreedyKind) -> Vec<Inst> {
        let mut insts = Vec::new();
        insts.extend(self.compile_repeat_count(ast, count));
        insts.extend(self.compile_star(ast, greedy));
        insts
    }

    fn compile_repeat_range(
        &mut self,
        ast: &Ast,
        min: u32,
        max: u32,
        greedy: &GreedyKind,
    ) -> Vec<Inst> {
        let mut child_insts = self.compile_root(&ast.children[0]);
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

        let mut repeat_insts = self.compile_repeat_count(ast, min);
        repeat_insts.reverse();
        insts.extend(repeat_insts);

        insts.reverse();
        insts
    }

    fn compile_match(kind: &MatchKind) -> Vec<Inst> {
        match kind {
            MatchKind::Any => [Inst::MatchCharAny].into(),
            MatchKind::Char(c) => [Inst::MatchChar(*c)].into(),
            MatchKind::Range(_, _) => unreachable!(),
        }
    }

    fn compile_position(position: &PositionKind) -> Vec<Inst> {
        match position {
            PositionKind::SoL => [Inst::MatchPosSOL].into(),
            PositionKind::EoL => [Inst::MatchPosEOL].into(),
        }
    }
}
