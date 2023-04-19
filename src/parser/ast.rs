#[derive(Debug, PartialEq)]
pub struct Ast {
    pub kind: AstKind,
    pub children: Vec<Ast>,
}

#[derive(Debug, PartialEq)]
pub enum AstKind {
    NonCapureGroup,
    CaptureGroup(usize),
    Union,
    IncludeSet,
    ExcludeSet,
    Star(GreedyKind),
    Plus(GreedyKind),
    Option(GreedyKind),
    Repeat(RepeatKind, RepeatKind, GreedyKind),
    Match(MatchKind),
    Position(PositionKind),
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
