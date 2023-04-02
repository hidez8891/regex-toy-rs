#[derive(Debug, Clone)]
pub enum Inst {
    Fail,
    Success,
    ConsumeRead,
    Jmp(isize),
    Split(isize, isize),
    Char(char),
    Any,
    PosSOL,
    PosEOL,
    JmpIfInclude(char, char, isize),
    SkipReadIfExclude(char, char),
}
