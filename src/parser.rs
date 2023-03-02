use std::iter::Peekable;
use std::vec::IntoIter;

// reference
// https://stackoverflow.com/questions/265457/regex-bnf-grammar
// https://www2.cs.sfu.ca/~cameron/Teaching/384/99-3/regexp-plg.html

const META_CHARS: [char; 12] = ['(', ')', '|', '*', '+', '?', '.', '[', ']', '^', '-', '$'];

#[derive(Debug, PartialEq)]
pub enum SyntaxKind {
    Group,                  // '(' abc ')'
    Union,                  // abc '|' abc
    ManyStar,               // a '*'
    ManyPlus,               // a '+'
    Option,                 // a '?'
    MatchAny,               // '.'
    Match(char),            // a
    PositiveSet,            // '[' a b c ']
    NegativeSet,            // '[^' a b c ']
    MatchRange(char, char), // a '-' z
    MatchSOL,               // ^
    MatchEOL,               // $
    None,
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
    pub fn new(pattern: String) -> Self {
        Parser {
            stream: pattern
                .chars()
                .collect::<Vec<char>>()
                .into_iter()
                .peekable(),
        }
    }

    pub fn parse(&mut self) -> Result<SyntaxNode, String> {
        let node = self.parse_root()?;
        match self.stream.next() {
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
                Ok(SyntaxNode {
                    kind: SyntaxKind::ManyStar,
                    children: vec![node],
                })
            }
            Some('+') => {
                self.stream.next();
                Ok(SyntaxNode {
                    kind: SyntaxKind::ManyPlus,
                    children: vec![node],
                })
            }
            Some('?') => {
                self.stream.next();

                Ok(SyntaxNode {
                    kind: SyntaxKind::Option,
                    children: vec![node],
                })
            }
            _ => Ok(node),
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

        let is_negative = self.stream.next_if_eq(&'^').is_some();
        let node = self.parse_set_items()?;

        match self.stream.next() {
            Some(']') => {
                if is_negative {
                    Ok(SyntaxNode {
                        kind: SyntaxKind::NegativeSet,
                        children: node.children,
                    })
                } else {
                    Ok(SyntaxNode {
                        kind: SyntaxKind::PositiveSet,
                        children: node.children,
                    })
                }
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

                if let (SyntaxKind::Match(a), SyntaxKind::Match(b)) = (node.kind, rhs.kind) {
                    if a > b {
                        return Err(format!("out of range order [{}-{}]", a, b));
                    }

                    Ok(SyntaxNode {
                        kind: SyntaxKind::MatchRange(a, b),
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
            kind: SyntaxKind::MatchAny,
            children: vec![],
        })
    }

    fn parse_sol(&mut self) -> Result<SyntaxNode, String> {
        self.stream.next(); // consume '^'

        Ok(SyntaxNode {
            kind: SyntaxKind::MatchSOL,
            children: vec![],
        })
    }

    fn parse_eol(&mut self) -> Result<SyntaxNode, String> {
        self.stream.next(); // consume '$'

        Ok(SyntaxNode {
            kind: SyntaxKind::MatchEOL,
            children: vec![],
        })
    }

    fn parse_char(&mut self) -> Result<SyntaxNode, String> {
        match self.stream.peek() {
            Some('\\') => self.parse_metachar(),
            Some(c) if !META_CHARS.contains(c) => {
                let c = self.stream.next().unwrap();
                Ok(SyntaxNode {
                    kind: SyntaxKind::Match(c),
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
                        kind: SyntaxKind::Match(c),
                        children: vec![],
                    })
                } else {
                    Err(format!("unsupport escape sequence: \\{}", c))
                }
            }
            _ => Err("escape sequence is empty".to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(pattern: &str) -> Result<SyntaxNode, String> {
        let mut parser = Parser::new(pattern.to_owned());
        parser.parse()
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

    #[test]
    fn match_char() {
        let src = "abc";
        let expect = Ok(make2(
            SyntaxKind::Group,
            vec![
                make1(SyntaxKind::Match('a')),
                make1(SyntaxKind::Match('b')),
                make1(SyntaxKind::Match('c')),
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
                make1(SyntaxKind::Match('a')),
                make1(SyntaxKind::Match('+')),
                make1(SyntaxKind::Match('c')),
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
                    make1(SyntaxKind::Match('a')),
                    make1(SyntaxKind::MatchAny),
                    make1(SyntaxKind::Match('c')),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "a.";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![make1(SyntaxKind::Match('a')), make1(SyntaxKind::MatchAny)],
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
                make1(SyntaxKind::MatchSOL),
                make1(SyntaxKind::Match('a')),
                make1(SyntaxKind::Match('b')),
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
                make1(SyntaxKind::Match('a')),
                make1(SyntaxKind::Match('b')),
                make1(SyntaxKind::MatchEOL),
            ],
        ));

        assert_eq!(run(src), expect);
    }

    #[test]
    fn group() {
        {
            let src = "a(bc)d";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(
                        SyntaxKind::Group,
                        vec![make1(SyntaxKind::Match('b')), make1(SyntaxKind::Match('c'))],
                    ),
                    make1(SyntaxKind::Match('d')),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "a(bc)";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(
                        SyntaxKind::Group,
                        vec![make1(SyntaxKind::Match('b')), make1(SyntaxKind::Match('c'))],
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
                        make1(SyntaxKind::Match('a')),
                        make1(SyntaxKind::Match('b')),
                        make1(SyntaxKind::Match('c')),
                    ],
                ),
                make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match('d')),
                        make1(SyntaxKind::Match('e')),
                        make1(SyntaxKind::Match('f')),
                    ],
                ),
                make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match('g')),
                        make1(SyntaxKind::Match('h')),
                        make1(SyntaxKind::Match('i')),
                    ],
                ),
            ],
        ));

        assert_eq!(run(src), expect);
    }

    #[test]
    fn many_star() {
        {
            let src = "ab*c";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(SyntaxKind::ManyStar, vec![make1(SyntaxKind::Match('b'))]),
                    make1(SyntaxKind::Match('c')),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "ab*";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(SyntaxKind::ManyStar, vec![make1(SyntaxKind::Match('b'))]),
                ],
            ));

            assert_eq!(run(src), expect);
        }
    }

    #[test]
    fn many_plus() {
        {
            let src = "ab+c";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(SyntaxKind::ManyPlus, vec![make1(SyntaxKind::Match('b'))]),
                    make1(SyntaxKind::Match('c')),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "ab+";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(SyntaxKind::ManyPlus, vec![make1(SyntaxKind::Match('b'))]),
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
                    make1(SyntaxKind::Match('a')),
                    make2(SyntaxKind::Option, vec![make1(SyntaxKind::Match('b'))]),
                    make1(SyntaxKind::Match('c')),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "ab?";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(SyntaxKind::Option, vec![make1(SyntaxKind::Match('b'))]),
                ],
            ));

            assert_eq!(run(src), expect);
        }
    }

    #[test]
    fn positive_set() {
        {
            let src = "a[b-z]d";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(
                        SyntaxKind::PositiveSet,
                        vec![make1(SyntaxKind::MatchRange('b', 'z'))],
                    ),
                    make1(SyntaxKind::Match('d')),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "[b-z]";
            let expect = Ok(make2(
                SyntaxKind::PositiveSet,
                vec![make1(SyntaxKind::MatchRange('b', 'z'))],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "[bcd]";
            let expect = Ok(make2(
                SyntaxKind::PositiveSet,
                vec![
                    make1(SyntaxKind::Match('b')),
                    make1(SyntaxKind::Match('c')),
                    make1(SyntaxKind::Match('d')),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "a[bc-yz]d";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(
                        SyntaxKind::PositiveSet,
                        vec![
                            make1(SyntaxKind::Match('b')),
                            make1(SyntaxKind::MatchRange('c', 'y')),
                            make1(SyntaxKind::Match('z')),
                        ],
                    ),
                    make1(SyntaxKind::Match('d')),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "[z-z]";
            let expect = Ok(make2(
                SyntaxKind::PositiveSet,
                vec![make1(SyntaxKind::MatchRange('z', 'z'))],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "[z-b]";
            assert_eq!(run(src).is_err(), true);
        }
    }

    #[test]
    fn negative_set() {
        {
            let src = "a[^b-z]d";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(
                        SyntaxKind::NegativeSet,
                        vec![make1(SyntaxKind::MatchRange('b', 'z'))],
                    ),
                    make1(SyntaxKind::Match('d')),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "[^b-z]";
            let expect = Ok(make2(
                SyntaxKind::NegativeSet,
                vec![make1(SyntaxKind::MatchRange('b', 'z'))],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "[^bcd]";
            let expect = Ok(make2(
                SyntaxKind::NegativeSet,
                vec![
                    make1(SyntaxKind::Match('b')),
                    make1(SyntaxKind::Match('c')),
                    make1(SyntaxKind::Match('d')),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "a[^bc-yz]d";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(
                        SyntaxKind::NegativeSet,
                        vec![
                            make1(SyntaxKind::Match('b')),
                            make1(SyntaxKind::MatchRange('c', 'y')),
                            make1(SyntaxKind::Match('z')),
                        ],
                    ),
                    make1(SyntaxKind::Match('d')),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "[^z-z]";
            let expect = Ok(make2(
                SyntaxKind::NegativeSet,
                vec![make1(SyntaxKind::MatchRange('z', 'z'))],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "[^z-b]";
            assert_eq!(run(src).is_err(), true);
        }
    }

    #[test]
    fn pattern001() {
        let src = r"[a-zA-Z0-9_\.\+\-]+@[a-zA-Z0-9_\.]+[a-zA-Z]+";
        let expect = Ok(make2(
            SyntaxKind::Group,
            vec![
                make2(
                    SyntaxKind::ManyPlus,
                    vec![make2(
                        SyntaxKind::PositiveSet,
                        vec![
                            make1(SyntaxKind::MatchRange('a', 'z')),
                            make1(SyntaxKind::MatchRange('A', 'Z')),
                            make1(SyntaxKind::MatchRange('0', '9')),
                            make1(SyntaxKind::Match('_')),
                            make1(SyntaxKind::Match('.')),
                            make1(SyntaxKind::Match('+')),
                            make1(SyntaxKind::Match('-')),
                        ],
                    )],
                ),
                make1(SyntaxKind::Match('@')),
                make2(
                    SyntaxKind::ManyPlus,
                    vec![make2(
                        SyntaxKind::PositiveSet,
                        vec![
                            make1(SyntaxKind::MatchRange('a', 'z')),
                            make1(SyntaxKind::MatchRange('A', 'Z')),
                            make1(SyntaxKind::MatchRange('0', '9')),
                            make1(SyntaxKind::Match('_')),
                            make1(SyntaxKind::Match('.')),
                        ],
                    )],
                ),
                make2(
                    SyntaxKind::ManyPlus,
                    vec![make2(
                        SyntaxKind::PositiveSet,
                        vec![
                            make1(SyntaxKind::MatchRange('a', 'z')),
                            make1(SyntaxKind::MatchRange('A', 'Z')),
                        ],
                    )],
                ),
            ],
        ));

        assert_eq!(run(src), expect);
    }
}
