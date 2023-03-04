use std::iter::Peekable;
use std::vec::IntoIter;

// reference
// https://stackoverflow.com/questions/265457/regex-bnf-grammar
// https://www2.cs.sfu.ca/~cameron/Teaching/384/99-3/regexp-plg.html

// syntax (like BNF)
//
// root      = union
// union     = concat ( '|' concat ) +
// concat    = basic +
// basic     = element ( ( '*' | '+' | '?' | '{' repeat '}' ) '?' ? ) ?
// repeat    = number ( ',' number ? ) ?
// element   = '(' group ')' | '[' set ']' | '.' | '^' | '$' | char
// group     = root
// set       = '^' ? set-items
// set-items = set-item +
// set-item  = char ( '-' char ) ?

const META_CHARS: [char; 15] = [
    '|', // union
    '*', // aster
    '+', // plus
    '?', // option or -shortest
    ',', // repeat range separator
    '-', // set range separator
    '^', // position Start-of-Line
    '$', // position End-of-Line
    '.', // any char
    '{', '}', // repeat brackets
    '(', ')', // group brackets
    '[', ']', // set brackets
];

#[derive(Debug, PartialEq)]
pub enum SyntaxKind {
    Group,        // '(' abc ')'
    Union,        // abc '|' abc
    Set(SetKind), // '[' a b c ']'
    Longest(RepeatKind),
    Shortest(RepeatKind),
    Match(MatchKind),
    Pos(PosKind),
    None,
}

#[derive(Debug, PartialEq)]
pub enum SetKind {
    Positive, // "[" abc "]"
    Negative, // "[^" abc "]"
}

#[derive(Debug, PartialEq)]
pub enum RepeatKind {
    Star,                  // a '*'
    Plus,                  // a '+'
    Repeat(u32),           // a '{' 10 '}'
    RepeatMin(u32),        // a '{' 1 ','    '}'
    RepeatRange(u32, u32), // a '{' 1 ',' 10 '}'
    Option,                // a '?'
}

#[derive(Debug, PartialEq)]
pub enum MatchKind {
    Any,               // '.'
    Char(char),        // a
    Range(char, char), // a '-' z
}

#[derive(Debug, PartialEq)]
pub enum PosKind {
    SOL, // '^'
    EOL, // '$'
}

#[derive(Debug, PartialEq)]
pub struct SyntaxNode {
    pub kind: SyntaxKind,
    pub children: Vec<SyntaxNode>,
}

pub struct Parser {
    stream: Peekable<IntoIter<char>>,
}

impl Parser {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(pattern: &str) -> Result<SyntaxNode, String> {
        let mut parser = Parser {
            stream: pattern
                .chars()
                .collect::<Vec<char>>()
                .into_iter()
                .peekable(),
        };

        let node = parser.parse_root()?;
        match parser.stream.next() {
            Some(c) => Err(format!("parse is failed: {}", c)),
            None => Ok(node),
        }
    }

    fn parse_root(&mut self) -> Result<SyntaxNode, String> {
        self.parse_union()
    }

    fn parse_union(&mut self) -> Result<SyntaxNode, String> {
        let node = self.parse_concat()?;
        if node.kind == SyntaxKind::None {
            return Ok(node);
        }

        match self.stream.peek() {
            Some('|') => {
                let mut children = vec![node];

                while let Some('|') = self.stream.peek() {
                    self.stream.next();

                    let rhs = self.parse_concat()?;
                    if rhs.kind == SyntaxKind::None {
                        return Err("missing right term of the union operator".to_owned());
                    }
                    children.push(rhs);
                }

                Ok(SyntaxNode {
                    kind: SyntaxKind::Union,
                    children,
                })
            }
            _ => Ok(node),
        }
    }

    fn parse_concat(&mut self) -> Result<SyntaxNode, String> {
        let mut children = Vec::new();
        loop {
            let node = self.parse_basic()?;

            match node.kind {
                SyntaxKind::None => break,
                _ => {
                    children.push(node);
                }
            }
        }

        match children.len() {
            0 => Ok(SyntaxNode {
                kind: SyntaxKind::None,
                children: vec![],
            }),
            1 => Ok(children.pop().unwrap()),
            _ => Ok(SyntaxNode {
                kind: SyntaxKind::Group,
                children,
            }),
        }
    }

    fn parse_basic(&mut self) -> Result<SyntaxNode, String> {
        let node = self.parse_element()?;
        if node.kind == SyntaxKind::None {
            return Ok(node);
        }

        match self.stream.peek() {
            Some('*') => {
                self.stream.next();

                let kind = match self.stream.next_if_eq(&'?') {
                    Some(_) => SyntaxKind::Shortest(RepeatKind::Star),
                    None => SyntaxKind::Longest(RepeatKind::Star),
                };

                Ok(SyntaxNode {
                    kind,
                    children: vec![node],
                })
            }
            Some('+') => {
                self.stream.next();

                let kind = match self.stream.next_if_eq(&'?') {
                    Some(_) => SyntaxKind::Shortest(RepeatKind::Plus),
                    None => SyntaxKind::Longest(RepeatKind::Plus),
                };

                Ok(SyntaxNode {
                    kind,
                    children: vec![node],
                })
            }
            Some('?') => {
                self.stream.next();

                let kind = match self.stream.next_if_eq(&'?') {
                    Some(_) => SyntaxKind::Shortest(RepeatKind::Option),
                    None => SyntaxKind::Longest(RepeatKind::Option),
                };

                Ok(SyntaxNode {
                    kind,
                    children: vec![node],
                })
            }
            Some('{') => {
                let repeat_kind = self.parse_repeat_kind()?;

                let kind = match self.stream.next_if_eq(&'?') {
                    Some(_) => SyntaxKind::Shortest(repeat_kind),
                    None => SyntaxKind::Longest(repeat_kind),
                };

                Ok(SyntaxNode {
                    kind,
                    children: vec![node],
                })
            }
            _ => Ok(node),
        }
    }

    fn parse_repeat_kind(&mut self) -> Result<RepeatKind, String> {
        self.stream.next(); // consume '{'

        let start = self
            .parse_number()
            .ok_or("repeat count is empty".to_owned())?;

        if self.stream.next_if_eq(&'}').is_some() {
            return Ok(RepeatKind::Repeat(start));
        }

        if self.stream.next_if_eq(&',').is_none() {
            match self.stream.next() {
                Some(c) => {
                    return Err(format!("repeat operator want ',', get {}", c));
                }
                _ => {
                    return Err("repeat operator want ',', get EOL".to_owned());
                }
            }
        }

        let end = self.parse_number().unwrap_or(u32::MAX);
        if start > end {
            return Err(format!("out of repeat order {{{},{}}}", start, end));
        }

        let repeat_kind = match (start, end) {
            (_, u32::MAX) => RepeatKind::RepeatMin(start),
            _ => RepeatKind::RepeatRange(start, end),
        };

        match self.stream.next() {
            Some('}') => Ok(repeat_kind),
            Some(c) => Err(format!("unmatched opening curly brackes, get '{}'", c)),
            _ => Err("unmatched opening curly brackes, get EOL".to_owned()),
        }
    }

    fn parse_element(&mut self) -> Result<SyntaxNode, String> {
        match self.stream.peek() {
            Some('(') => self.parse_group(),
            Some('[') => self.parse_set(),
            Some('.') => self.parse_anychar(),
            Some('^') => self.parse_sol(),
            Some('$') => self.parse_eol(),
            _ => self.parse_char(),
        }
    }

    fn parse_group(&mut self) -> Result<SyntaxNode, String> {
        self.stream.next(); // consume '('

        let node = self.parse_root()?;
        if node.kind == SyntaxKind::None {
            return Ok(node);
        }

        match self.stream.next() {
            Some(')') => Ok(node),
            Some(c) => Err(format!("unmatched opening parentheses, get '{}'", c)),
            _ => Err("unmatched opening parentheses, get EOL".to_owned()),
        }
    }

    fn parse_set(&mut self) -> Result<SyntaxNode, String> {
        self.stream.next(); // consume '['

        let is_positive = self.stream.next_if_eq(&'^').is_none();
        let node = self.parse_set_items()?;

        match self.stream.next() {
            Some(']') => {
                let set_kind = match is_positive {
                    true => SetKind::Positive,
                    false => SetKind::Negative,
                };

                Ok(SyntaxNode {
                    kind: SyntaxKind::Set(set_kind),
                    children: node.children,
                })
            }
            Some(c) => Err(format!("unmatched opening brackets, get '{}'", c)),
            _ => Err("unmatched opening brackets, get EOL".to_owned()),
        }
    }

    fn parse_set_items(&mut self) -> Result<SyntaxNode, String> {
        let mut children = Vec::new();
        loop {
            let node = self.parse_set_item()?;

            match node.kind {
                SyntaxKind::None => break,
                _ => {
                    children.push(node);
                }
            }
        }

        match children.len() {
            0 => Err("set items are empty".to_owned()),
            _ => Ok(SyntaxNode {
                kind: SyntaxKind::Group,
                children,
            }),
        }
    }

    fn parse_set_item(&mut self) -> Result<SyntaxNode, String> {
        let node = self.parse_char()?;
        if node.kind == SyntaxKind::None {
            return Ok(node);
        }

        match self.stream.peek() {
            Some('-') => {
                self.stream.next();

                let rhs = self.parse_char()?;
                if rhs.kind == SyntaxKind::None {
                    return Err("missing range end character".to_owned());
                }

                if let (
                    SyntaxKind::Match(MatchKind::Char(a)),
                    SyntaxKind::Match(MatchKind::Char(b)),
                ) = (node.kind, rhs.kind)
                {
                    if a > b {
                        return Err(format!("out of range order [{}-{}]", a, b));
                    }

                    Ok(SyntaxNode {
                        kind: SyntaxKind::Match(MatchKind::Range(a, b)),
                        children: vec![],
                    })
                } else {
                    unreachable!()
                }
            }
            _ => Ok(node),
        }
    }

    fn parse_anychar(&mut self) -> Result<SyntaxNode, String> {
        self.stream.next(); // consume '.'

        Ok(SyntaxNode {
            kind: SyntaxKind::Match(MatchKind::Any),
            children: vec![],
        })
    }

    fn parse_sol(&mut self) -> Result<SyntaxNode, String> {
        self.stream.next(); // consume '^'

        Ok(SyntaxNode {
            kind: SyntaxKind::Pos(PosKind::SOL),
            children: vec![],
        })
    }

    fn parse_eol(&mut self) -> Result<SyntaxNode, String> {
        self.stream.next(); // consume '$'

        Ok(SyntaxNode {
            kind: SyntaxKind::Pos(PosKind::EOL),
            children: vec![],
        })
    }

    fn parse_char(&mut self) -> Result<SyntaxNode, String> {
        match self.stream.peek() {
            Some('\\') => self.parse_metachar(),
            Some(c) if !META_CHARS.contains(c) => {
                let c = self.stream.next().unwrap();
                Ok(SyntaxNode {
                    kind: SyntaxKind::Match(MatchKind::Char(c)),
                    children: vec![],
                })
            }
            _ => Ok(SyntaxNode {
                kind: SyntaxKind::None,
                children: vec![],
            }),
        }
    }

    fn parse_metachar(&mut self) -> Result<SyntaxNode, String> {
        self.stream.next(); // consume '\\'

        match self.stream.next() {
            Some(c) => {
                if META_CHARS.contains(&c) {
                    Ok(SyntaxNode {
                        kind: SyntaxKind::Match(MatchKind::Char(c)),
                        children: vec![],
                    })
                } else {
                    Err(format!("unsupport escape sequence: \\{}", c))
                }
            }
            _ => Err("escape sequence is empty".to_owned()),
        }
    }

    fn parse_number(&mut self) -> Option<u32> {
        let mut num = String::new();
        while let Some(c) = self.stream.next_if(|c| c.is_digit(10)) {
            num.push(c);
        }

        if num.is_empty() {
            None
        } else {
            Some(num.parse().unwrap())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(pattern: &str) -> Result<SyntaxNode, String> {
        Parser::new(pattern)
    }

    fn make1(kind: SyntaxKind) -> SyntaxNode {
        SyntaxNode {
            kind,
            children: vec![],
        }
    }

    fn make2(kind: SyntaxKind, children: Vec<SyntaxNode>) -> SyntaxNode {
        SyntaxNode { kind, children }
    }

    #[cfg(test)]
    mod basic_match {
        use super::*;

        #[test]
        fn match_char() {
            let src = "abc";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match(MatchKind::Char('a'))),
                    make1(SyntaxKind::Match(MatchKind::Char('b'))),
                    make1(SyntaxKind::Match(MatchKind::Char('c'))),
                ],
            ));

            assert_eq!(run(src), expect);
        }

        #[test]
        fn match_metachar() {
            let src = r"a\+c";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match(MatchKind::Char('a'))),
                    make1(SyntaxKind::Match(MatchKind::Char('+'))),
                    make1(SyntaxKind::Match(MatchKind::Char('c'))),
                ],
            ));

            assert_eq!(run(src), expect);
        }

        #[test]
        fn match_any() {
            {
                let src = "a.c";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make1(SyntaxKind::Match(MatchKind::Any)),
                        make1(SyntaxKind::Match(MatchKind::Char('c'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "a.";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make1(SyntaxKind::Match(MatchKind::Any)),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
        }

        #[test]
        fn match_sol() {
            let src = "^ab";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Pos(PosKind::SOL)),
                    make1(SyntaxKind::Match(MatchKind::Char('a'))),
                    make1(SyntaxKind::Match(MatchKind::Char('b'))),
                ],
            ));

            assert_eq!(run(src), expect);
        }

        #[test]
        fn match_eol() {
            let src = "ab$";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match(MatchKind::Char('a'))),
                    make1(SyntaxKind::Match(MatchKind::Char('b'))),
                    make1(SyntaxKind::Pos(PosKind::EOL)),
                ],
            ));

            assert_eq!(run(src), expect);
        }
    }

    #[test]
    fn group() {
        {
            let src = "a(bc)d";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match(MatchKind::Char('a'))),
                    make2(
                        SyntaxKind::Group,
                        vec![
                            make1(SyntaxKind::Match(MatchKind::Char('b'))),
                            make1(SyntaxKind::Match(MatchKind::Char('c'))),
                        ],
                    ),
                    make1(SyntaxKind::Match(MatchKind::Char('d'))),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "a(bc)";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match(MatchKind::Char('a'))),
                    make2(
                        SyntaxKind::Group,
                        vec![
                            make1(SyntaxKind::Match(MatchKind::Char('b'))),
                            make1(SyntaxKind::Match(MatchKind::Char('c'))),
                        ],
                    ),
                ],
            ));

            assert_eq!(run(src), expect);
        }
    }

    #[test]
    fn union() {
        let src = "abc|def|ghi";
        let expect = Ok(make2(
            SyntaxKind::Union,
            vec![
                make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make1(SyntaxKind::Match(MatchKind::Char('b'))),
                        make1(SyntaxKind::Match(MatchKind::Char('c'))),
                    ],
                ),
                make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('d'))),
                        make1(SyntaxKind::Match(MatchKind::Char('e'))),
                        make1(SyntaxKind::Match(MatchKind::Char('f'))),
                    ],
                ),
                make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('g'))),
                        make1(SyntaxKind::Match(MatchKind::Char('h'))),
                        make1(SyntaxKind::Match(MatchKind::Char('i'))),
                    ],
                ),
            ],
        ));

        assert_eq!(run(src), expect);
    }

    #[cfg(test)]
    mod longest {
        use super::*;

        #[test]
        fn star() {
            {
                let src = "ab*c";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Longest(RepeatKind::Star),
                            vec![make1(SyntaxKind::Match(MatchKind::Char('b')))],
                        ),
                        make1(SyntaxKind::Match(MatchKind::Char('c'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "ab*";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Longest(RepeatKind::Star),
                            vec![make1(SyntaxKind::Match(MatchKind::Char('b')))],
                        ),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
        }

        #[test]
        fn plus() {
            {
                let src = "ab+c";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Longest(RepeatKind::Plus),
                            vec![make1(SyntaxKind::Match(MatchKind::Char('b')))],
                        ),
                        make1(SyntaxKind::Match(MatchKind::Char('c'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "ab+";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Longest(RepeatKind::Plus),
                            vec![make1(SyntaxKind::Match(MatchKind::Char('b')))],
                        ),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
        }

        #[test]
        fn option() {
            {
                let src = "ab?c";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Longest(RepeatKind::Option),
                            vec![make1(SyntaxKind::Match(MatchKind::Char('b')))],
                        ),
                        make1(SyntaxKind::Match(MatchKind::Char('c'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "ab?";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Longest(RepeatKind::Option),
                            vec![make1(SyntaxKind::Match(MatchKind::Char('b')))],
                        ),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
        }

        #[cfg(test)]
        mod repeat {
            use super::*;

            #[test]
            fn repeat() {
                {
                    let src = "a{10}";
                    let expect = Ok(make2(
                        SyntaxKind::Longest(RepeatKind::Repeat(10)),
                        vec![make1(SyntaxKind::Match(MatchKind::Char('a')))],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "abc{10}";
                    let expect = Ok(make2(
                        SyntaxKind::Group,
                        vec![
                            make1(SyntaxKind::Match(MatchKind::Char('a'))),
                            make1(SyntaxKind::Match(MatchKind::Char('b'))),
                            make2(
                                SyntaxKind::Longest(RepeatKind::Repeat(10)),
                                vec![make1(SyntaxKind::Match(MatchKind::Char('c')))],
                            ),
                        ],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "(abc){10}";
                    let expect = Ok(make2(
                        SyntaxKind::Longest(RepeatKind::Repeat(10)),
                        vec![make2(
                            SyntaxKind::Group,
                            vec![
                                make1(SyntaxKind::Match(MatchKind::Char('a'))),
                                make1(SyntaxKind::Match(MatchKind::Char('b'))),
                                make1(SyntaxKind::Match(MatchKind::Char('c'))),
                            ],
                        )],
                    ));

                    assert_eq!(run(src), expect);
                }
            }

            #[test]
            fn repeat_min() {
                {
                    let src = "a{1,}";
                    let expect = Ok(make2(
                        SyntaxKind::Longest(RepeatKind::RepeatMin(1)),
                        vec![make1(SyntaxKind::Match(MatchKind::Char('a')))],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "abc{1,}";
                    let expect = Ok(make2(
                        SyntaxKind::Group,
                        vec![
                            make1(SyntaxKind::Match(MatchKind::Char('a'))),
                            make1(SyntaxKind::Match(MatchKind::Char('b'))),
                            make2(
                                SyntaxKind::Longest(RepeatKind::RepeatMin(1)),
                                vec![make1(SyntaxKind::Match(MatchKind::Char('c')))],
                            ),
                        ],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "(abc){1,}";
                    let expect = Ok(make2(
                        SyntaxKind::Longest(RepeatKind::RepeatMin(1)),
                        vec![make2(
                            SyntaxKind::Group,
                            vec![
                                make1(SyntaxKind::Match(MatchKind::Char('a'))),
                                make1(SyntaxKind::Match(MatchKind::Char('b'))),
                                make1(SyntaxKind::Match(MatchKind::Char('c'))),
                            ],
                        )],
                    ));

                    assert_eq!(run(src), expect);
                }
            }

            #[test]
            fn repeat_range() {
                {
                    let src = "a{1,10}";
                    let expect = Ok(make2(
                        SyntaxKind::Longest(RepeatKind::RepeatRange(1, 10)),
                        vec![make1(SyntaxKind::Match(MatchKind::Char('a')))],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "abc{1,10}";
                    let expect = Ok(make2(
                        SyntaxKind::Group,
                        vec![
                            make1(SyntaxKind::Match(MatchKind::Char('a'))),
                            make1(SyntaxKind::Match(MatchKind::Char('b'))),
                            make2(
                                SyntaxKind::Longest(RepeatKind::RepeatRange(1, 10)),
                                vec![make1(SyntaxKind::Match(MatchKind::Char('c')))],
                            ),
                        ],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "(abc){1,10}";
                    let expect = Ok(make2(
                        SyntaxKind::Longest(RepeatKind::RepeatRange(1, 10)),
                        vec![make2(
                            SyntaxKind::Group,
                            vec![
                                make1(SyntaxKind::Match(MatchKind::Char('a'))),
                                make1(SyntaxKind::Match(MatchKind::Char('b'))),
                                make1(SyntaxKind::Match(MatchKind::Char('c'))),
                            ],
                        )],
                    ));

                    assert_eq!(run(src), expect);
                }
            }
        }
    }

    #[cfg(test)]
    mod shortest {
        use super::*;

        #[test]
        fn star() {
            {
                let src = "ab*?c";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Shortest(RepeatKind::Star),
                            vec![make1(SyntaxKind::Match(MatchKind::Char('b')))],
                        ),
                        make1(SyntaxKind::Match(MatchKind::Char('c'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "ab*?";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Shortest(RepeatKind::Star),
                            vec![make1(SyntaxKind::Match(MatchKind::Char('b')))],
                        ),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
        }

        #[test]
        fn plus() {
            {
                let src = "ab+?c";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Shortest(RepeatKind::Plus),
                            vec![make1(SyntaxKind::Match(MatchKind::Char('b')))],
                        ),
                        make1(SyntaxKind::Match(MatchKind::Char('c'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "ab+?";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Shortest(RepeatKind::Plus),
                            vec![make1(SyntaxKind::Match(MatchKind::Char('b')))],
                        ),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
        }
    }

    #[cfg(test)]
    mod set {
        use super::*;

        #[test]
        fn positive() {
            {
                let src = "a[b-z]d";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Set(SetKind::Positive),
                            vec![make1(SyntaxKind::Match(MatchKind::Range('b', 'z')))],
                        ),
                        make1(SyntaxKind::Match(MatchKind::Char('d'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[b-z]";
                let expect = Ok(make2(
                    SyntaxKind::Set(SetKind::Positive),
                    vec![make1(SyntaxKind::Match(MatchKind::Range('b', 'z')))],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[bcd]";
                let expect = Ok(make2(
                    SyntaxKind::Set(SetKind::Positive),
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('b'))),
                        make1(SyntaxKind::Match(MatchKind::Char('c'))),
                        make1(SyntaxKind::Match(MatchKind::Char('d'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "a[bc-yz]d";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Set(SetKind::Positive),
                            vec![
                                make1(SyntaxKind::Match(MatchKind::Char('b'))),
                                make1(SyntaxKind::Match(MatchKind::Range('c', 'y'))),
                                make1(SyntaxKind::Match(MatchKind::Char('z'))),
                            ],
                        ),
                        make1(SyntaxKind::Match(MatchKind::Char('d'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[z-z]";
                let expect = Ok(make2(
                    SyntaxKind::Set(SetKind::Positive),
                    vec![make1(SyntaxKind::Match(MatchKind::Range('z', 'z')))],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[z-b]";
                assert_eq!(run(src).is_err(), true);
            }
        }

        #[test]
        fn negative() {
            {
                let src = "a[^b-z]d";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Set(SetKind::Negative),
                            vec![make1(SyntaxKind::Match(MatchKind::Range('b', 'z')))],
                        ),
                        make1(SyntaxKind::Match(MatchKind::Char('d'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[^b-z]";
                let expect = Ok(make2(
                    SyntaxKind::Set(SetKind::Negative),
                    vec![make1(SyntaxKind::Match(MatchKind::Range('b', 'z')))],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[^bcd]";
                let expect = Ok(make2(
                    SyntaxKind::Set(SetKind::Negative),
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('b'))),
                        make1(SyntaxKind::Match(MatchKind::Char('c'))),
                        make1(SyntaxKind::Match(MatchKind::Char('d'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "a[^bc-yz]d";
                let expect = Ok(make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match(MatchKind::Char('a'))),
                        make2(
                            SyntaxKind::Set(SetKind::Negative),
                            vec![
                                make1(SyntaxKind::Match(MatchKind::Char('b'))),
                                make1(SyntaxKind::Match(MatchKind::Range('c', 'y'))),
                                make1(SyntaxKind::Match(MatchKind::Char('z'))),
                            ],
                        ),
                        make1(SyntaxKind::Match(MatchKind::Char('d'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[^z-z]";
                let expect = Ok(make2(
                    SyntaxKind::Set(SetKind::Negative),
                    vec![make1(SyntaxKind::Match(MatchKind::Range('z', 'z')))],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[^z-b]";
                assert_eq!(run(src).is_err(), true);
            }
        }
    }

    #[test]
    fn pattern001() {
        let src = r"[a-zA-Z0-9_\.\+\-]+@[a-zA-Z0-9_\.]+[a-zA-Z]+";
        let expect = Ok(make2(
            SyntaxKind::Group,
            vec![
                make2(
                    SyntaxKind::Longest(RepeatKind::Plus),
                    vec![make2(
                        SyntaxKind::Set(SetKind::Positive),
                        vec![
                            make1(SyntaxKind::Match(MatchKind::Range('a', 'z'))),
                            make1(SyntaxKind::Match(MatchKind::Range('A', 'Z'))),
                            make1(SyntaxKind::Match(MatchKind::Range('0', '9'))),
                            make1(SyntaxKind::Match(MatchKind::Char('_'))),
                            make1(SyntaxKind::Match(MatchKind::Char('.'))),
                            make1(SyntaxKind::Match(MatchKind::Char('+'))),
                            make1(SyntaxKind::Match(MatchKind::Char('-'))),
                        ],
                    )],
                ),
                make1(SyntaxKind::Match(MatchKind::Char('@'))),
                make2(
                    SyntaxKind::Longest(RepeatKind::Plus),
                    vec![make2(
                        SyntaxKind::Set(SetKind::Positive),
                        vec![
                            make1(SyntaxKind::Match(MatchKind::Range('a', 'z'))),
                            make1(SyntaxKind::Match(MatchKind::Range('A', 'Z'))),
                            make1(SyntaxKind::Match(MatchKind::Range('0', '9'))),
                            make1(SyntaxKind::Match(MatchKind::Char('_'))),
                            make1(SyntaxKind::Match(MatchKind::Char('.'))),
                        ],
                    )],
                ),
                make2(
                    SyntaxKind::Longest(RepeatKind::Plus),
                    vec![make2(
                        SyntaxKind::Set(SetKind::Positive),
                        vec![
                            make1(SyntaxKind::Match(MatchKind::Range('a', 'z'))),
                            make1(SyntaxKind::Match(MatchKind::Range('A', 'Z'))),
                        ],
                    )],
                ),
            ],
        ));

        assert_eq!(run(src), expect);
    }
}
