use std::iter::Peekable;
use std::vec::IntoIter;

#[derive(Debug, PartialEq)]
pub enum SyntaxKind {
    Group,
    Select,
    ZeroLoop,
    MoreLoop,
    Option,
    MatchAny,
    Match(char),
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
        self.parse_root()
    }

    fn parse_root(&mut self) -> Result<SyntaxNode, String> {
        self.parse_select()
    }

    fn parse_select(&mut self) -> Result<SyntaxNode, String> {
        let node = self.parse_conjunct()?;

        match self.stream.peek() {
            Some('|') => {
                self.stream.next();
                let rhs = self.parse_select()?;

                match rhs.kind {
                    SyntaxKind::Select => {
                        let mut children = vec![node];
                        children.extend(rhs.children);

                        Ok(SyntaxNode {
                            kind: SyntaxKind::Select,
                            children,
                        })
                    }
                    _ => Ok(SyntaxNode {
                        kind: SyntaxKind::Select,
                        children: vec![node, rhs],
                    }),
                }
            }
            _ => Ok(node),
        }
    }

    fn parse_conjunct(&mut self) -> Result<SyntaxNode, String> {
        let mut children = Vec::new();
        loop {
            match self.stream.peek() {
                Some('|') | Some(')') | None => {
                    break;
                }
                _ => {
                    let node = self.parse_loop()?;
                    children.push(node);
                }
            }
        }

        Ok(SyntaxNode {
            kind: SyntaxKind::Group,
            children,
        })
    }

    fn parse_loop(&mut self) -> Result<SyntaxNode, String> {
        let node = self.parse_option()?;

        match self.stream.peek() {
            Some('*') => {
                self.stream.next();
                Ok(SyntaxNode {
                    kind: SyntaxKind::ZeroLoop,
                    children: vec![node],
                })
            }
            Some('+') => {
                self.stream.next();
                Ok(SyntaxNode {
                    kind: SyntaxKind::MoreLoop,
                    children: vec![node],
                })
            }
            _ => Ok(node),
        }
    }

    fn parse_option(&mut self) -> Result<SyntaxNode, String> {
        let node = self.parse_match()?;

        match self.stream.peek() {
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

    fn parse_match(&mut self) -> Result<SyntaxNode, String> {
        match self.stream.peek() {
            Some('.') => {
                self.stream.next();
                Ok(SyntaxNode {
                    kind: SyntaxKind::MatchAny,
                    children: vec![],
                })
            }
            Some('(') => {
                self.stream.next();
                let node = self.parse_root()?;

                match self.stream.next() {
                    Some(')') => Ok(node),
                    Some(c) => Err(format!("closing parentheses do not match, get '{}'", c)),
                    _ => Err("closing parentheses do not match, get EOL".to_owned()),
                }
            }
            Some(_) => {
                let c = self.stream.next().unwrap();
                Ok(SyntaxNode {
                    kind: SyntaxKind::Match(c),
                    children: vec![],
                })
            }
            _ => {
                unreachable!()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn basic() {
        let src = "abc".to_owned();

        let mut parser = Parser::new(src);
        let result = parser.parse();

        let expect = Ok(make2(
            SyntaxKind::Group,
            vec![
                make1(SyntaxKind::Match('a')),
                make1(SyntaxKind::Match('b')),
                make1(SyntaxKind::Match('c')),
            ],
        ));

        assert_eq!(result, expect);
    }

    #[test]
    fn select() {
        let src = "abc|def|ghi".to_owned();

        let mut parser = Parser::new(src);
        let result = parser.parse();

        let expect = Ok(make2(
            SyntaxKind::Select,
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

        assert_eq!(result, expect);
    }

    #[test]
    fn zero_loop() {
        let src = "ab*c".to_owned();

        let mut parser = Parser::new(src);
        let result = parser.parse();

        let expect = Ok(make2(
            SyntaxKind::Group,
            vec![
                make1(SyntaxKind::Match('a')),
                make2(SyntaxKind::ZeroLoop, vec![make1(SyntaxKind::Match('b'))]),
                make1(SyntaxKind::Match('c')),
            ],
        ));

        assert_eq!(result, expect);
    }

    #[test]
    fn more_loop() {
        let src = "ab+c".to_owned();

        let mut parser = Parser::new(src);
        let result = parser.parse();

        let expect = Ok(make2(
            SyntaxKind::Group,
            vec![
                make1(SyntaxKind::Match('a')),
                make2(SyntaxKind::MoreLoop, vec![make1(SyntaxKind::Match('b'))]),
                make1(SyntaxKind::Match('c')),
            ],
        ));

        assert_eq!(result, expect);
    }

    #[test]
    fn option() {
        let src = "ab?c".to_owned();

        let mut parser = Parser::new(src);
        let result = parser.parse();

        let expect = Ok(make2(
            SyntaxKind::Group,
            vec![
                make1(SyntaxKind::Match('a')),
                make2(SyntaxKind::Option, vec![make1(SyntaxKind::Match('b'))]),
                make1(SyntaxKind::Match('c')),
            ],
        ));

        assert_eq!(result, expect);
    }

    #[test]
    fn match_any() {
        let src = "a.c".to_owned();

        let mut parser = Parser::new(src);
        let result = parser.parse();

        let expect = Ok(make2(
            SyntaxKind::Group,
            vec![
                make1(SyntaxKind::Match('a')),
                make1(SyntaxKind::MatchAny),
                make1(SyntaxKind::Match('c')),
            ],
        ));

        assert_eq!(result, expect);
    }

    #[test]
    fn match_group() {
        let src = "a(bc)d".to_owned();

        let mut parser = Parser::new(src);
        let result = parser.parse();

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

        assert_eq!(result, expect);
    }

    #[test]
    fn pattern001() {
        let src = "(https?|ftp):(exp.)?+".to_owned();

        let mut parser = Parser::new(src);
        let result = parser.parse();

        let expect = Ok(make2(
            SyntaxKind::Group,
            vec![
                make2(
                    SyntaxKind::Select,
                    vec![
                        make2(
                            SyntaxKind::Group,
                            vec![
                                make1(SyntaxKind::Match('h')),
                                make1(SyntaxKind::Match('t')),
                                make1(SyntaxKind::Match('t')),
                                make1(SyntaxKind::Match('p')),
                                make2(SyntaxKind::Option, vec![make1(SyntaxKind::Match('s'))]),
                            ],
                        ),
                        make2(
                            SyntaxKind::Group,
                            vec![
                                make1(SyntaxKind::Match('f')),
                                make1(SyntaxKind::Match('t')),
                                make1(SyntaxKind::Match('p')),
                            ],
                        ),
                    ],
                ),
                make1(SyntaxKind::Match(':')),
                make2(
                    SyntaxKind::MoreLoop,
                    vec![make2(
                        SyntaxKind::Option,
                        vec![make2(
                            SyntaxKind::Group,
                            vec![
                                make1(SyntaxKind::Match('e')),
                                make1(SyntaxKind::Match('x')),
                                make1(SyntaxKind::Match('p')),
                                make1(SyntaxKind::MatchAny),
                            ],
                        )],
                    )],
                ),
            ],
        ));

        assert_eq!(result, expect);
    }
}
