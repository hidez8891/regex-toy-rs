#[derive(Debug, Clone)]
pub(crate) enum Inst {
    Fail,
    Success,
    Seek(isize),
    Jmp(isize),
    JmpIfTrue(isize),
    JmpIfFalse(isize),
    Split(isize, isize),
    MatchChar(char),
    MatchCharAny,
    MatchPosSOL,
    MatchPosEOL,
    CheckInclude(char, char),
    CheckExclude(char, char),
}
