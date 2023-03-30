#[derive(Debug, PartialEq)]
pub struct Ast {
    pub kind: AstKind,
    pub children: Vec<Ast>,
}

#[derive(Debug, PartialEq)]
pub enum AstKind {
    Group,
    Union,
    IncludeSet,
    ExcludeSet,
    Star(GreedyKind),
    Plus(GreedyKind),
    Option(GreedyKind),
    Repeat(RepeatKind, RepeatKind, GreedyKind),
    Match(MatchKind),
    Position(PositionKind),
    None,
}

#[derive(Debug, PartialEq)]
pub enum GreedyKind {
    Greedy,
    NonGreedy,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RepeatKind {
    Num(u32),
    Infinity,
}

#[derive(Debug, PartialEq)]
pub enum MatchKind {
    Any,               // '.'
    Char(char),        // a
    Range(char, char), // a - z
}

#[derive(Debug, PartialEq)]
pub enum PositionKind {
    SoL, // '^'
    EoL, // '$'
}

impl Ast {
    pub fn new(kind: AstKind) -> Self {
        Ast {
            kind,
            children: vec![],
        }
    }

    pub fn set_greedy(&mut self, greedy: GreedyKind) {
        use AstKind::*;

        match &self.kind {
            Star(_) => self.kind = Star(greedy),
            Plus(_) => self.kind = Plus(greedy),
            Repeat(n, m, _) => self.kind = Repeat(*n, *m, greedy),
            _ => { /* nothing */ }
        }
    }
}
