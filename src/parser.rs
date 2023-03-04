use std::iter::Peekable;
use std::vec::IntoIter;

// reference
// https://stackoverflow.com/questions/265457/regex-bnf-grammar
// https://www2.cs.sfu.ca/~cameron/Teaching/384/99-3/regexp-plg.html

// syntax (like BNF)
//
// root      = union
// union     = concat ( '|' concat ) +
// concat    = ( basic ) +
// basic     = element ( '*?' | '*' | '+?' | '+' | '?' | '{' repeat '}' ) ?
// repeat    = number ',' number ? | number
// element   = '(' group ')' | '[' set ']' | '.' | '^' | '$' | char
// group     = root
// set       = '^' ? set-items
// set-items = ( set-item ) +
// set-item  = char ( '-' char ) ?

const META_CHARS: [char; 12] = ['(', ')', '|', '*', '+', '?', '.', '[', ']', '^', '-', '$'];

#[derive(Debug, PartialEq)]
pub enum SyntaxKind {
    Group,                  // '(' abc ')'
    Union,                  // abc '|' abc
    LongestStar,            // a '*'
    LongestPlus,            // a '+'
    ShortestStar,           // a '*?'
    ShortestPlus,           // a '+?'
    Repeat(u32),            // a '{' 10 '}'
    RepeatMin(u32),         // a '{' 1 ','    '}'
    RepeatRange(u32, u32),  // a '{' 1 ',' 10 '}'
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
                if self.stream.next_if_eq(&'?').is_some() {
                    Ok(SyntaxNode {
                        kind: SyntaxKind::ShortestStar,
                        children: vec![node],
                    })
                } else {
                    Ok(SyntaxNode {
                        kind: SyntaxKind::LongestStar,
                        children: vec![node],
                    })
                }
            }
            Some('+') => {
                self.stream.next();
                if self.stream.next_if_eq(&'?').is_some() {
                    Ok(SyntaxNode {
                        kind: SyntaxKind::ShortestPlus,
                        children: vec![node],
                    })
                } else {
                    Ok(SyntaxNode {
                        kind: SyntaxKind::LongestPlus,
                        children: vec![node],
                    })
                }
            }
            Some('?') => {
                self.stream.next();

                Ok(SyntaxNode {
                    kind: SyntaxKind::Option,
                    children: vec![node],
                })
            }
            Some('{') => self.parse_repeat(node),
            _ => Ok(node),
        }
    }

    fn parse_repeat(&mut self, node: SyntaxNode) -> Result<SyntaxNode, String> {
        self.stream.next(); // consume '{'

        let start = self
            .parse_number()
            .ok_or("repeat count is empty".to_owned())?;

        if self.stream.next_if_eq(&'}').is_some() {
            return Ok(SyntaxNode {
                kind: SyntaxKind::Repeat(start),
                children: vec![node],
            });
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

        let kind = match (start, end) {
            (_, u32::MAX) => SyntaxKind::RepeatMin(start),
            _ => SyntaxKind::RepeatRange(start, end),
        };

        match self.stream.next() {
            Some('}') => Ok(SyntaxNode {
                kind: kind,
                children: vec![node],
            }),
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
    fn longest_star() {
        {
            let src = "ab*c";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(SyntaxKind::LongestStar, vec![make1(SyntaxKind::Match('b'))]),
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
                    make2(SyntaxKind::LongestStar, vec![make1(SyntaxKind::Match('b'))]),
                ],
            ));

            assert_eq!(run(src), expect);
        }
    }

    #[test]
    fn longest_plus() {
        {
            let src = "ab+c";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(SyntaxKind::LongestPlus, vec![make1(SyntaxKind::Match('b'))]),
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
                    make2(SyntaxKind::LongestPlus, vec![make1(SyntaxKind::Match('b'))]),
                ],
            ));

            assert_eq!(run(src), expect);
        }
    }

    #[test]
    fn shortest_star() {
        {
            let src = "ab*?c";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(
                        SyntaxKind::ShortestStar,
                        vec![make1(SyntaxKind::Match('b'))],
                    ),
                    make1(SyntaxKind::Match('c')),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "ab*?";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(
                        SyntaxKind::ShortestStar,
                        vec![make1(SyntaxKind::Match('b'))],
                    ),
                ],
            ));

            assert_eq!(run(src), expect);
        }
    }

    #[test]
    fn shortest_plus() {
        {
            let src = "ab+?c";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(
                        SyntaxKind::ShortestPlus,
                        vec![make1(SyntaxKind::Match('b'))],
                    ),
                    make1(SyntaxKind::Match('c')),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "ab+?";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make2(
                        SyntaxKind::ShortestPlus,
                        vec![make1(SyntaxKind::Match('b'))],
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
    fn repeat() {
        {
            let src = "a{10}";
            let expect = Ok(make2(
                SyntaxKind::Repeat(10),
                vec![make1(SyntaxKind::Match('a'))],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "abc{10}";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make1(SyntaxKind::Match('b')),
                    make2(SyntaxKind::Repeat(10), vec![make1(SyntaxKind::Match('c'))]),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "(abc){10}";
            let expect = Ok(make2(
                SyntaxKind::Repeat(10),
                vec![make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match('a')),
                        make1(SyntaxKind::Match('b')),
                        make1(SyntaxKind::Match('c')),
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
                SyntaxKind::RepeatMin(1),
                vec![make1(SyntaxKind::Match('a'))],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "abc{1,}";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make1(SyntaxKind::Match('b')),
                    make2(
                        SyntaxKind::RepeatMin(1),
                        vec![make1(SyntaxKind::Match('c'))],
                    ),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "(abc){1,}";
            let expect = Ok(make2(
                SyntaxKind::RepeatMin(1),
                vec![make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match('a')),
                        make1(SyntaxKind::Match('b')),
                        make1(SyntaxKind::Match('c')),
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
                SyntaxKind::RepeatRange(1, 10),
                vec![make1(SyntaxKind::Match('a'))],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "abc{1,10}";
            let expect = Ok(make2(
                SyntaxKind::Group,
                vec![
                    make1(SyntaxKind::Match('a')),
                    make1(SyntaxKind::Match('b')),
                    make2(
                        SyntaxKind::RepeatRange(1, 10),
                        vec![make1(SyntaxKind::Match('c'))],
                    ),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "(abc){1,10}";
            let expect = Ok(make2(
                SyntaxKind::RepeatRange(1, 10),
                vec![make2(
                    SyntaxKind::Group,
                    vec![
                        make1(SyntaxKind::Match('a')),
                        make1(SyntaxKind::Match('b')),
                        make1(SyntaxKind::Match('c')),
                    ],
                )],
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
                    SyntaxKind::LongestPlus,
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
                    SyntaxKind::LongestPlus,
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
                    SyntaxKind::LongestPlus,
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
