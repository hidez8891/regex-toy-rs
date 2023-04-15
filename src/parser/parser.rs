use std::iter::Peekable;
use std::num::ParseIntError;
use std::vec::IntoIter;

use super::{
    ast::{AstKind, GreedyKind, MatchKind, PositionKind, RepeatKind},
    Ast,
};

const META_CHARS: [char; 15] = [
    '|', // union
    '*', // star
    '+', // plus
    '?', // option or non-greedy
    ',', // repeat range separator
    '-', // set range separator
    '^', // position Start-of-Line
    '$', // position End-of-Line
    '.', // any match
    '{', '}', // repeat brackets
    '(', ')', // group brackets
    '[', ']', // set brackets
];

pub(crate) struct Parser {
    stream: Peekable<IntoIter<char>>,
}

impl Parser {
    pub fn parse(pattern: &str) -> Result<Ast, String> {
        let mut parser = Parser {
            stream: pattern
                .chars()
                .collect::<Vec<char>>()
                .into_iter()
                .peekable(),
        };

        let ast = parser.parse_concat()?;
        match parser.stream.next() {
            Some(c) => Err(format!("parse is failed: {}", c)),
            None => Ok(ast),
        }
    }

    fn parse_concat(&mut self) -> Result<Ast, String> {
        let mut children = vec![];
        let mut ast = None;

        loop {
            match self.stream.peek() {
                Some('(') => {
                    if let Some(node) = ast {
                        children.push(node);
                    }
                    ast = Some(self.parse_group()?);
                }
                Some('[') => {
                    if let Some(node) = ast {
                        children.push(node);
                    }
                    ast = Some(self.parse_set()?);
                }
                Some('{') => {
                    if ast.is_none() {
                        return Err(format!("ERROR: repeat target is empty"));
                    }
                    ast = Some(self.parse_repeat(ast.unwrap())?);
                }
                Some('|') => {
                    if ast.is_none() {
                        return Err(format!("ERROR: union target is empty"));
                    }

                    children.push(ast.unwrap());
                    let lhs = Ast {
                        kind: AstKind::Group,
                        children,
                    };
                    children = vec![];

                    ast = Some(self.parse_union(lhs)?);
                }
                Some('*') => {
                    if ast.is_none() {
                        return Err(format!("ERROR: star target is empty"));
                    }
                    ast = Some(self.parse_star(ast.unwrap())?);
                }
                Some('+') => {
                    if ast.is_none() {
                        return Err(format!("ERROR: plus target is empty"));
                    }
                    ast = Some(self.parse_plus(ast.unwrap())?);
                }
                Some('?') => {
                    if ast.is_none() {
                        return Err(format!("ERROR: option target is empty"));
                    }
                    ast = Some(self.parse_option(ast.unwrap())?);
                }
                Some('^') | Some('$') => {
                    if let Some(node) = ast {
                        children.push(node);
                    }
                    ast = Some(self.parse_position()?);
                }
                Some('\\') => {
                    if let Some(node) = ast {
                        children.push(node);
                    }
                    ast = Some(self.parse_metachar()?);
                }
                Some('.') => {
                    if let Some(node) = ast {
                        children.push(node);
                    }
                    ast = Some(self.parse_any()?);
                }
                Some(c) if META_CHARS.contains(c) => {
                    break; // end loop
                }
                Some(_) => {
                    if let Some(node) = ast {
                        children.push(node);
                    }
                    ast = Some(self.parse_char()?);
                }
                None => {
                    break; // EOL, end loop
                }
            }
        }

        if let Some(node) = ast {
            children.push(node);
        }

        return Ok(Ast {
            kind: AstKind::Group,
            children,
        });
    }

    fn parse_set_items(&mut self) -> Result<Vec<Ast>, String> {
        let mut children = vec![];
        let mut ast = None;

        loop {
            match self.stream.peek() {
                Some('\\') => {
                    if let Some(node) = ast {
                        children.push(node);
                    }
                    ast = Some(self.parse_metachar()?);
                }
                Some('-') => {
                    if ast.is_none() {
                        return Err(format!("ERROR: char-range start is empty"));
                    }
                    children.push(self.parse_char_range(ast.unwrap())?);
                    ast = None;
                }
                Some(c) if META_CHARS.contains(c) => {
                    break; // end loop
                }
                Some(_) => {
                    if let Some(node) = ast {
                        children.push(node);
                    }
                    ast = Some(self.parse_char()?);
                }
                None => {
                    break; // EOL, end loop
                }
            }
        }

        if let Some(node) = ast {
            children.push(node);
        }

        return Ok(children);
    }

    fn parse_group(&mut self) -> Result<Ast, String> {
        if self.stream.next_if_eq(&'(').is_none() {
            return Err(format!("ERROR: want group open token"));
        }

        let ast = self.parse_concat()?;

        if self.stream.next_if_eq(&')').is_none() {
            return Err(format!("ERROR: want group close token"));
        }

        return Ok(ast);
    }

    fn parse_set(&mut self) -> Result<Ast, String> {
        if self.stream.next_if_eq(&'[').is_none() {
            return Err(format!("ERROR: want set open token"));
        }

        let is_positive = self.stream.next_if_eq(&'^').is_none();
        let children = self.parse_set_items()?;

        if self.stream.next_if_eq(&']').is_none() {
            return Err(format!("ERROR: want set close token"));
        }

        if is_positive {
            return Ok(Ast {
                kind: AstKind::IncludeSet,
                children,
            });
        } else {
            return Ok(Ast {
                kind: AstKind::ExcludeSet,
                children,
            });
        }
    }

    fn parse_repeat(&mut self, lhs: Ast) -> Result<Ast, String> {
        if self.stream.next_if_eq(&'{').is_none() {
            return Err(format!("ERROR: want repeat open token"));
        }

        let mut min = RepeatKind::Num(0);
        let mut max = RepeatKind::Infinity;

        if self.stream.next_if_eq(&',').is_some() {
            // pattern : {,n}
            max = RepeatKind::Num(self.parse_number()?);
        } else {
            min = RepeatKind::Num(self.parse_number()?);

            if self.stream.next_if_eq(&',').is_none() {
                // pattern : {n}
                max = min
            } else {
                if self.stream.peek() != Some(&'}') {
                    // pattern : {n, m}
                    max = RepeatKind::Num(self.parse_number()?);
                } else {
                    // pattern : {n,}
                }
            }
        }

        if self.stream.next_if_eq(&'}').is_none() {
            return Err(format!("ERROR: want repeat close token"));
        }

        match (min, max) {
            (RepeatKind::Num(n), RepeatKind::Num(m)) if n > m => {
                return Err(format!("ERROR: repeat range invalid {{{},{}}}", n, m));
            }
            _ => { /* OK */ }
        }

        let greedy = match self.stream.next_if_eq(&'?') {
            Some(_) => GreedyKind::NonGreedy,
            None => GreedyKind::Greedy,
        };

        return Ok(Ast {
            kind: AstKind::Repeat(min, max, greedy),
            children: vec![lhs],
        });
    }

    fn parse_union(&mut self, lhs: Ast) -> Result<Ast, String> {
        if self.stream.next_if_eq(&'|').is_none() {
            return Err(format!("ERROR: want union token"));
        }

        let mut rhs = self.parse_concat()?;
        if rhs.children.is_empty() {
            return Err(format!("ERROR: union right token is empty"));
        }

        let ast = match rhs.children[0].kind {
            AstKind::None => {
                return Err(format!("ERROR: union right token is empty"));
            }
            AstKind::Union => {
                assert!(rhs.children.len() == 1);

                let mut children = vec![lhs];
                children.append(&mut rhs.children[0].children);

                Ast {
                    kind: AstKind::Union,
                    children,
                }
            }
            _ => Ast {
                kind: AstKind::Union,
                children: vec![lhs, rhs],
            },
        };

        return Ok(ast);
    }

    fn parse_star(&mut self, lhs: Ast) -> Result<Ast, String> {
        if self.stream.next_if_eq(&'*').is_none() {
            return Err(format!("ERROR: want star token"));
        }

        let greedy = match self.stream.next_if_eq(&'?') {
            Some(_) => GreedyKind::NonGreedy,
            None => GreedyKind::Greedy,
        };

        return Ok(Ast {
            kind: AstKind::Star(greedy),
            children: vec![lhs],
        });
    }

    fn parse_plus(&mut self, lhs: Ast) -> Result<Ast, String> {
        if self.stream.next_if_eq(&'+').is_none() {
            return Err(format!("ERROR: want plus token"));
        }

        let greedy = match self.stream.next_if_eq(&'?') {
            Some(_) => GreedyKind::NonGreedy,
            None => GreedyKind::Greedy,
        };

        return Ok(Ast {
            kind: AstKind::Plus(greedy),
            children: vec![lhs],
        });
    }

    fn parse_option(&mut self, lhs: Ast) -> Result<Ast, String> {
        if self.stream.next_if_eq(&'?').is_none() {
            return Err(format!("ERROR: want option token"));
        }

        let greedy = match self.stream.next_if_eq(&'?') {
            Some(_) => GreedyKind::NonGreedy,
            None => GreedyKind::Greedy,
        };

        return Ok(Ast {
            kind: AstKind::Option(greedy),
            children: vec![lhs],
        });
    }

    fn parse_position(&mut self) -> Result<Ast, String> {
        let pos = match self.stream.next() {
            Some('^') => PositionKind::SoL,
            Some('$') => PositionKind::EoL,
            Some(c) => return Err(format!("ERROR: unsupport position '{}'", c)),
            None => return Err(format!("ERROR: want position token, get EOL")),
        };

        return Ok(Ast {
            kind: AstKind::Position(pos),
            children: vec![],
        });
    }

    fn parse_metachar(&mut self) -> Result<Ast, String> {
        if self.stream.next_if_eq(&'\\').is_none() {
            return Err(format!("ERROR: want \\ token"));
        }

        match self.stream.next() {
            Some(c) if META_CHARS.contains(&c) => {
                return Ok(Ast {
                    kind: AstKind::Match(MatchKind::Char(c)),
                    children: vec![],
                });
            }
            Some(c) if c == '\\' => {
                return Ok(Ast {
                    kind: AstKind::Match(MatchKind::Char('\\')),
                    children: vec![],
                });
            }
            Some(c) => {
                return Err(format!("ERROR: unsupport control sequence '\\{}'", c));
            }
            None => {
                return Err(format!("ERROR: want control sequence, get EOL"));
            }
        }
    }

    fn parse_any(&mut self) -> Result<Ast, String> {
        if self.stream.next_if_eq(&'.').is_none() {
            return Err(format!("ERROR: want . token"));
        }

        return Ok(Ast {
            kind: AstKind::Match(MatchKind::Any),
            children: vec![],
        });
    }

    fn parse_char(&mut self) -> Result<Ast, String> {
        match self.stream.next() {
            Some(c) => {
                return Ok(Ast {
                    kind: AstKind::Match(MatchKind::Char(c)),
                    children: vec![],
                });
            }
            None => {
                return Err(format!("ERROR: want control sequence, get EOL"));
            }
        }
    }

    fn parse_char_range(&mut self, lhs: Ast) -> Result<Ast, String> {
        if self.stream.next_if_eq(&'-').is_none() {
            return Err(format!("ERROR: want char-range '-' token"));
        }

        let rhs = match self.stream.peek() {
            Some('\\') => self.parse_metachar()?,
            Some(c) if META_CHARS.contains(c) => {
                return Err(format!("ERROR: want char-range end, get {}", c));
            }
            Some(_) => self.parse_char()?,
            None => {
                return Err(format!("ERROR: want char-range end, get EOL"));
            }
        };

        let a = if let AstKind::Match(MatchKind::Char(c)) = lhs.kind {
            c
        } else {
            unreachable!()
        };

        let b = if let AstKind::Match(MatchKind::Char(c)) = rhs.kind {
            c
        } else {
            unreachable!()
        };

        if a > b {
            return Err(format!("ERROR: invalid char-range [{}-{}]", a, b));
        }

        return Ok(Ast {
            kind: AstKind::Match(MatchKind::Range(a, b)),
            children: vec![],
        });
    }

    fn parse_number(&mut self) -> Result<u32, String> {
        let mut num = String::new();
        while let Some(c) = self.stream.next_if(|c| c.is_ascii_digit()) {
            num.push(c);
        }

        num.parse()
            .or_else(|err: ParseIntError| Err(err.to_string()))
    }
}
