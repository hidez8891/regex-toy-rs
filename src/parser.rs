use std::iter::Peekable;
use std::vec::IntoIter;

use self::ast::*;

mod ast;

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

pub struct Parser {
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

        let ast = parser.parse_root()?;
        match parser.stream.next() {
            Some(c) => Err(format!("parse is failed: {}", c)),
            None => Ok(ast),
        }
    }

    fn parse_root(&mut self) -> Result<Ast, String> {
        self.parse_union()
    }

    fn parse_union(&mut self) -> Result<Ast, String> {
        let ast = self.parse_concat()?;
        if ast.kind == AstKind::None {
            return Ok(ast);
        }

        match self.stream.peek() {
            Some('|') => {
                let mut children = vec![ast];

                while let Some('|') = self.stream.peek() {
                    self.stream.next();

                    let rhs = self.parse_concat()?;
                    if rhs.kind == AstKind::None {
                        return Err("missing right term of the union operator".to_owned());
                    }
                    children.push(rhs);
                }

                Ok(Ast {
                    kind: AstKind::Union,
                    children,
                })
            }
            _ => Ok(ast),
        }
    }

    fn parse_concat(&mut self) -> Result<Ast, String> {
        let mut children = Vec::new();
        loop {
            let ast = self.parse_basic()?;

            match ast.kind {
                AstKind::None => break,
                _ => {
                    children.push(ast);
                }
            }
        }

        match children.len() {
            0 => Ok(Ast {
                kind: AstKind::None,
                children: vec![],
            }),
            1 => Ok(children.pop().unwrap()),
            _ => Ok(Ast {
                kind: AstKind::Group,
                children,
            }),
        }
    }

    fn parse_basic(&mut self) -> Result<Ast, String> {
        let ast = self.parse_element()?;
        if ast.kind == AstKind::None {
            return Ok(ast);
        }

        let kind = match self.stream.peek() {
            Some('*') => {
                self.stream.next();

                match self.stream.next_if_eq(&'?') {
                    Some(_) => AstKind::Star(GreedyKind::NonGreedy),
                    None => AstKind::Star(GreedyKind::Greedy),
                }
            }
            Some('+') => {
                self.stream.next();

                match self.stream.next_if_eq(&'?') {
                    Some(_) => AstKind::Plus(GreedyKind::NonGreedy),
                    None => AstKind::Plus(GreedyKind::Greedy),
                }
            }
            Some('?') => {
                self.stream.next();

                match self.stream.next_if_eq(&'?') {
                    Some(_) => AstKind::Option(GreedyKind::NonGreedy),
                    None => AstKind::Option(GreedyKind::Greedy),
                }
            }
            Some('{') => {
                if let AstKind::Repeat(n, m, _) = self.parse_repeat_kind()? {
                    match self.stream.next_if_eq(&'?') {
                        Some(_) => AstKind::Repeat(n, m, GreedyKind::NonGreedy),
                        None => AstKind::Repeat(n, m, GreedyKind::Greedy),
                    }
                } else {
                    unreachable!()
                }
            }
            _ => {
                return Ok(ast);
            }
        };

        Ok(Ast {
            kind,
            children: vec![ast],
        })
    }

    fn parse_repeat_kind(&mut self) -> Result<AstKind, String> {
        use GreedyKind::*;
        use RepeatKind::*;

        self.stream.next(); // consume '{'

        let start = self
            .parse_number()
            .ok_or("repeat count is empty".to_owned())?;

        if self.stream.next_if_eq(&'}').is_some() {
            return Ok(AstKind::Repeat(Num(start), Num(start), Greedy));
        }

        if self.stream.next_if_eq(&',').is_none() {
            match self.stream.next() {
                Some(c) => {
                    return Err(format!("repeat operator want ',', get {}", c));
                }
                _ => {
                    return Err("repeat operator want ',', get EoL".to_owned());
                }
            }
        }

        let end = self.parse_number().unwrap_or(u32::MAX);
        if start > end {
            return Err(format!("out of repeat order {{{},{}}}", start, end));
        }

        let repeat_kind = match (start, end) {
            (_, u32::MAX) => AstKind::Repeat(Num(start), Infinity, Greedy),
            _ => AstKind::Repeat(Num(start), Num(end), Greedy),
        };

        match self.stream.next() {
            Some('}') => Ok(repeat_kind),
            Some(c) => Err(format!("unmatched opening curly brackes, get '{}'", c)),
            _ => Err("unmatched opening curly brackes, get EoL".to_owned()),
        }
    }

    fn parse_element(&mut self) -> Result<Ast, String> {
        match self.stream.peek() {
            Some('(') => self.parse_group(),
            Some('[') => self.parse_set(),
            Some('.') => self.parse_anychar(),
            Some('^') => self.parse_sol(),
            Some('$') => self.parse_eol(),
            _ => self.parse_char(),
        }
    }

    fn parse_group(&mut self) -> Result<Ast, String> {
        self.stream.next(); // consume '('

        let ast = self.parse_root()?;
        if ast.kind == AstKind::None {
            return Ok(ast);
        }

        match self.stream.next() {
            Some(')') => Ok(ast),
            Some(c) => Err(format!("unmatched opening parentheses, get '{}'", c)),
            _ => Err("unmatched opening parentheses, get EoL".to_owned()),
        }
    }

    fn parse_set(&mut self) -> Result<Ast, String> {
        self.stream.next(); // consume '['

        let is_positive = self.stream.next_if_eq(&'^').is_none();
        let ast = self.parse_set_items()?;

        match self.stream.next() {
            Some(']') => {
                let kind = match is_positive {
                    true => AstKind::IncludeSet,
                    false => AstKind::ExcludeSet,
                };

                Ok(Ast {
                    kind,
                    children: ast.children,
                })
            }
            Some(c) => Err(format!("unmatched opening brackets, get '{}'", c)),
            _ => Err("unmatched opening brackets, get EoL".to_owned()),
        }
    }

    fn parse_set_items(&mut self) -> Result<Ast, String> {
        let mut children = Vec::new();
        loop {
            let ast = self.parse_set_item()?;

            match ast.kind {
                AstKind::None => break,
                _ => {
                    children.push(ast);
                }
            }
        }

        match children.len() {
            0 => Err("set items are empty".to_owned()),
            _ => Ok(Ast {
                kind: AstKind::Group,
                children,
            }),
        }
    }

    fn parse_set_item(&mut self) -> Result<Ast, String> {
        let ast = self.parse_char()?;
        if ast.kind == AstKind::None {
            return Ok(ast);
        }

        match self.stream.peek() {
            Some('-') => {
                self.stream.next();

                let rhs = self.parse_char()?;
                if rhs.kind == AstKind::None {
                    return Err("missing range end character".to_owned());
                }

                if let (AstKind::Match(MatchKind::Char(a)), AstKind::Match(MatchKind::Char(b))) =
                    (ast.kind, rhs.kind)
                {
                    if a > b {
                        return Err(format!("out of range order [{}-{}]", a, b));
                    }

                    Ok(Ast {
                        kind: AstKind::Match(MatchKind::Range(a, b)),
                        children: vec![],
                    })
                } else {
                    unreachable!()
                }
            }
            _ => Ok(ast),
        }
    }

    fn parse_anychar(&mut self) -> Result<Ast, String> {
        self.stream.next(); // consume '.'

        Ok(Ast {
            kind: AstKind::Match(MatchKind::Any),
            children: vec![],
        })
    }

    fn parse_sol(&mut self) -> Result<Ast, String> {
        self.stream.next(); // consume '^'

        Ok(Ast {
            kind: AstKind::Position(PositionKind::SoL),
            children: vec![],
        })
    }

    fn parse_eol(&mut self) -> Result<Ast, String> {
        self.stream.next(); // consume '$'

        Ok(Ast {
            kind: AstKind::Position(PositionKind::EoL),
            children: vec![],
        })
    }

    fn parse_char(&mut self) -> Result<Ast, String> {
        match self.stream.peek() {
            Some('\\') => self.parse_metachar(),
            Some(c) if !META_CHARS.contains(c) => {
                let c = self.stream.next().unwrap();
                Ok(Ast {
                    kind: AstKind::Match(MatchKind::Char(c)),
                    children: vec![],
                })
            }
            _ => Ok(Ast {
                kind: AstKind::None,
                children: vec![],
            }),
        }
    }

    fn parse_metachar(&mut self) -> Result<Ast, String> {
        self.stream.next(); // consume '\\'

        match self.stream.next() {
            Some(c) => {
                if META_CHARS.contains(&c) {
                    Ok(Ast {
                        kind: AstKind::Match(MatchKind::Char(c)),
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
        while let Some(c) = self.stream.next_if(|c| c.is_ascii_digit()) {
            num.push(c);
        }
        num.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(pattern: &str) -> Result<Ast, String> {
        Parser::parse(pattern)
    }

    fn make1(kind: AstKind) -> Ast {
        Ast {
            kind,
            children: vec![],
        }
    }

    fn make2(kind: AstKind, children: Vec<Ast>) -> Ast {
        Ast { kind, children }
    }

    #[cfg(test)]
    mod basic_match {
        use super::*;

        #[test]
        fn match_char() {
            let src = "abc";
            let expect = Ok(make2(
                AstKind::Group,
                vec![
                    make1(AstKind::Match(MatchKind::Char('a'))),
                    make1(AstKind::Match(MatchKind::Char('b'))),
                    make1(AstKind::Match(MatchKind::Char('c'))),
                ],
            ));

            assert_eq!(run(src), expect);
        }

        #[test]
        fn match_metachar() {
            let src = r"a\+c";
            let expect = Ok(make2(
                AstKind::Group,
                vec![
                    make1(AstKind::Match(MatchKind::Char('a'))),
                    make1(AstKind::Match(MatchKind::Char('+'))),
                    make1(AstKind::Match(MatchKind::Char('c'))),
                ],
            ));

            assert_eq!(run(src), expect);
        }

        #[test]
        fn match_any() {
            {
                let src = "a.c";
                let expect = Ok(make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make1(AstKind::Match(MatchKind::Any)),
                        make1(AstKind::Match(MatchKind::Char('c'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "a.";
                let expect = Ok(make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make1(AstKind::Match(MatchKind::Any)),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
        }

        #[test]
        fn match_sol() {
            let src = "^ab";
            let expect = Ok(make2(
                AstKind::Group,
                vec![
                    make1(AstKind::Position(PositionKind::SoL)),
                    make1(AstKind::Match(MatchKind::Char('a'))),
                    make1(AstKind::Match(MatchKind::Char('b'))),
                ],
            ));

            assert_eq!(run(src), expect);
        }

        #[test]
        fn match_eol() {
            let src = "ab$";
            let expect = Ok(make2(
                AstKind::Group,
                vec![
                    make1(AstKind::Match(MatchKind::Char('a'))),
                    make1(AstKind::Match(MatchKind::Char('b'))),
                    make1(AstKind::Position(PositionKind::EoL)),
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
                AstKind::Group,
                vec![
                    make1(AstKind::Match(MatchKind::Char('a'))),
                    make2(
                        AstKind::Group,
                        vec![
                            make1(AstKind::Match(MatchKind::Char('b'))),
                            make1(AstKind::Match(MatchKind::Char('c'))),
                        ],
                    ),
                    make1(AstKind::Match(MatchKind::Char('d'))),
                ],
            ));

            assert_eq!(run(src), expect);
        }
        {
            let src = "a(bc)";
            let expect = Ok(make2(
                AstKind::Group,
                vec![
                    make1(AstKind::Match(MatchKind::Char('a'))),
                    make2(
                        AstKind::Group,
                        vec![
                            make1(AstKind::Match(MatchKind::Char('b'))),
                            make1(AstKind::Match(MatchKind::Char('c'))),
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
            AstKind::Union,
            vec![
                make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make1(AstKind::Match(MatchKind::Char('b'))),
                        make1(AstKind::Match(MatchKind::Char('c'))),
                    ],
                ),
                make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('d'))),
                        make1(AstKind::Match(MatchKind::Char('e'))),
                        make1(AstKind::Match(MatchKind::Char('f'))),
                    ],
                ),
                make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('g'))),
                        make1(AstKind::Match(MatchKind::Char('h'))),
                        make1(AstKind::Match(MatchKind::Char('i'))),
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
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::Star(GreedyKind::Greedy),
                            vec![make1(AstKind::Match(MatchKind::Char('b')))],
                        ),
                        make1(AstKind::Match(MatchKind::Char('c'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "ab*";
                let expect = Ok(make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::Star(GreedyKind::Greedy),
                            vec![make1(AstKind::Match(MatchKind::Char('b')))],
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
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::Plus(GreedyKind::Greedy),
                            vec![make1(AstKind::Match(MatchKind::Char('b')))],
                        ),
                        make1(AstKind::Match(MatchKind::Char('c'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "ab+";
                let expect = Ok(make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::Plus(GreedyKind::Greedy),
                            vec![make1(AstKind::Match(MatchKind::Char('b')))],
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
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::Option(GreedyKind::Greedy),
                            vec![make1(AstKind::Match(MatchKind::Char('b')))],
                        ),
                        make1(AstKind::Match(MatchKind::Char('c'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "ab?";
                let expect = Ok(make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::Option(GreedyKind::Greedy),
                            vec![make1(AstKind::Match(MatchKind::Char('b')))],
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
                        AstKind::Repeat(
                            RepeatKind::Num(10),
                            RepeatKind::Num(10),
                            GreedyKind::Greedy,
                        ),
                        vec![make1(AstKind::Match(MatchKind::Char('a')))],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "abc{10}";
                    let expect = Ok(make2(
                        AstKind::Group,
                        vec![
                            make1(AstKind::Match(MatchKind::Char('a'))),
                            make1(AstKind::Match(MatchKind::Char('b'))),
                            make2(
                                AstKind::Repeat(
                                    RepeatKind::Num(10),
                                    RepeatKind::Num(10),
                                    GreedyKind::Greedy,
                                ),
                                vec![make1(AstKind::Match(MatchKind::Char('c')))],
                            ),
                        ],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "(abc){10}";
                    let expect = Ok(make2(
                        AstKind::Repeat(
                            RepeatKind::Num(10),
                            RepeatKind::Num(10),
                            GreedyKind::Greedy,
                        ),
                        vec![make2(
                            AstKind::Group,
                            vec![
                                make1(AstKind::Match(MatchKind::Char('a'))),
                                make1(AstKind::Match(MatchKind::Char('b'))),
                                make1(AstKind::Match(MatchKind::Char('c'))),
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
                        AstKind::Repeat(
                            RepeatKind::Num(1),
                            RepeatKind::Infinity,
                            GreedyKind::Greedy,
                        ),
                        vec![make1(AstKind::Match(MatchKind::Char('a')))],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "abc{1,}";
                    let expect = Ok(make2(
                        AstKind::Group,
                        vec![
                            make1(AstKind::Match(MatchKind::Char('a'))),
                            make1(AstKind::Match(MatchKind::Char('b'))),
                            make2(
                                AstKind::Repeat(
                                    RepeatKind::Num(1),
                                    RepeatKind::Infinity,
                                    GreedyKind::Greedy,
                                ),
                                vec![make1(AstKind::Match(MatchKind::Char('c')))],
                            ),
                        ],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "(abc){1,}";
                    let expect = Ok(make2(
                        AstKind::Repeat(
                            RepeatKind::Num(1),
                            RepeatKind::Infinity,
                            GreedyKind::Greedy,
                        ),
                        vec![make2(
                            AstKind::Group,
                            vec![
                                make1(AstKind::Match(MatchKind::Char('a'))),
                                make1(AstKind::Match(MatchKind::Char('b'))),
                                make1(AstKind::Match(MatchKind::Char('c'))),
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
                        AstKind::Repeat(
                            RepeatKind::Num(1),
                            RepeatKind::Num(10),
                            GreedyKind::Greedy,
                        ),
                        vec![make1(AstKind::Match(MatchKind::Char('a')))],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "abc{1,10}";
                    let expect = Ok(make2(
                        AstKind::Group,
                        vec![
                            make1(AstKind::Match(MatchKind::Char('a'))),
                            make1(AstKind::Match(MatchKind::Char('b'))),
                            make2(
                                AstKind::Repeat(
                                    RepeatKind::Num(1),
                                    RepeatKind::Num(10),
                                    GreedyKind::Greedy,
                                ),
                                vec![make1(AstKind::Match(MatchKind::Char('c')))],
                            ),
                        ],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "(abc){1,10}";
                    let expect = Ok(make2(
                        AstKind::Repeat(
                            RepeatKind::Num(1),
                            RepeatKind::Num(10),
                            GreedyKind::Greedy,
                        ),
                        vec![make2(
                            AstKind::Group,
                            vec![
                                make1(AstKind::Match(MatchKind::Char('a'))),
                                make1(AstKind::Match(MatchKind::Char('b'))),
                                make1(AstKind::Match(MatchKind::Char('c'))),
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
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::Star(GreedyKind::NonGreedy),
                            vec![make1(AstKind::Match(MatchKind::Char('b')))],
                        ),
                        make1(AstKind::Match(MatchKind::Char('c'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "ab*?";
                let expect = Ok(make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::Star(GreedyKind::NonGreedy),
                            vec![make1(AstKind::Match(MatchKind::Char('b')))],
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
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::Plus(GreedyKind::NonGreedy),
                            vec![make1(AstKind::Match(MatchKind::Char('b')))],
                        ),
                        make1(AstKind::Match(MatchKind::Char('c'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "ab+?";
                let expect = Ok(make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::Plus(GreedyKind::NonGreedy),
                            vec![make1(AstKind::Match(MatchKind::Char('b')))],
                        ),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
        }

        #[test]
        fn option() {
            {
                let src = "ab??c";
                let expect = Ok(make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::Option(GreedyKind::NonGreedy),
                            vec![make1(AstKind::Match(MatchKind::Char('b')))],
                        ),
                        make1(AstKind::Match(MatchKind::Char('c'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "ab??";
                let expect = Ok(make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::Option(GreedyKind::NonGreedy),
                            vec![make1(AstKind::Match(MatchKind::Char('b')))],
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
                    let src = "a{10}?";
                    let expect = Ok(make2(
                        AstKind::Repeat(
                            RepeatKind::Num(10),
                            RepeatKind::Num(10),
                            GreedyKind::NonGreedy,
                        ),
                        vec![make1(AstKind::Match(MatchKind::Char('a')))],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "abc{10}?";
                    let expect = Ok(make2(
                        AstKind::Group,
                        vec![
                            make1(AstKind::Match(MatchKind::Char('a'))),
                            make1(AstKind::Match(MatchKind::Char('b'))),
                            make2(
                                AstKind::Repeat(
                                    RepeatKind::Num(10),
                                    RepeatKind::Num(10),
                                    GreedyKind::NonGreedy,
                                ),
                                vec![make1(AstKind::Match(MatchKind::Char('c')))],
                            ),
                        ],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "(abc){10}?";
                    let expect = Ok(make2(
                        AstKind::Repeat(
                            RepeatKind::Num(10),
                            RepeatKind::Num(10),
                            GreedyKind::NonGreedy,
                        ),
                        vec![make2(
                            AstKind::Group,
                            vec![
                                make1(AstKind::Match(MatchKind::Char('a'))),
                                make1(AstKind::Match(MatchKind::Char('b'))),
                                make1(AstKind::Match(MatchKind::Char('c'))),
                            ],
                        )],
                    ));

                    assert_eq!(run(src), expect);
                }
            }

            #[test]
            fn repeat_min() {
                {
                    let src = "a{1,}?";
                    let expect = Ok(make2(
                        AstKind::Repeat(
                            RepeatKind::Num(1),
                            RepeatKind::Infinity,
                            GreedyKind::NonGreedy,
                        ),
                        vec![make1(AstKind::Match(MatchKind::Char('a')))],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "abc{1,}?";
                    let expect = Ok(make2(
                        AstKind::Group,
                        vec![
                            make1(AstKind::Match(MatchKind::Char('a'))),
                            make1(AstKind::Match(MatchKind::Char('b'))),
                            make2(
                                AstKind::Repeat(
                                    RepeatKind::Num(1),
                                    RepeatKind::Infinity,
                                    GreedyKind::NonGreedy,
                                ),
                                vec![make1(AstKind::Match(MatchKind::Char('c')))],
                            ),
                        ],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "(abc){1,}?";
                    let expect = Ok(make2(
                        AstKind::Repeat(
                            RepeatKind::Num(1),
                            RepeatKind::Infinity,
                            GreedyKind::NonGreedy,
                        ),
                        vec![make2(
                            AstKind::Group,
                            vec![
                                make1(AstKind::Match(MatchKind::Char('a'))),
                                make1(AstKind::Match(MatchKind::Char('b'))),
                                make1(AstKind::Match(MatchKind::Char('c'))),
                            ],
                        )],
                    ));

                    assert_eq!(run(src), expect);
                }
            }

            #[test]
            fn repeat_range() {
                {
                    let src = "a{1,10}?";
                    let expect = Ok(make2(
                        AstKind::Repeat(
                            RepeatKind::Num(1),
                            RepeatKind::Num(10),
                            GreedyKind::NonGreedy,
                        ),
                        vec![make1(AstKind::Match(MatchKind::Char('a')))],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "abc{1,10}?";
                    let expect = Ok(make2(
                        AstKind::Group,
                        vec![
                            make1(AstKind::Match(MatchKind::Char('a'))),
                            make1(AstKind::Match(MatchKind::Char('b'))),
                            make2(
                                AstKind::Repeat(
                                    RepeatKind::Num(1),
                                    RepeatKind::Num(10),
                                    GreedyKind::NonGreedy,
                                ),
                                vec![make1(AstKind::Match(MatchKind::Char('c')))],
                            ),
                        ],
                    ));

                    assert_eq!(run(src), expect);
                }
                {
                    let src = "(abc){1,10}?";
                    let expect = Ok(make2(
                        AstKind::Repeat(
                            RepeatKind::Num(1),
                            RepeatKind::Num(10),
                            GreedyKind::NonGreedy,
                        ),
                        vec![make2(
                            AstKind::Group,
                            vec![
                                make1(AstKind::Match(MatchKind::Char('a'))),
                                make1(AstKind::Match(MatchKind::Char('b'))),
                                make1(AstKind::Match(MatchKind::Char('c'))),
                            ],
                        )],
                    ));

                    assert_eq!(run(src), expect);
                }
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
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::IncludeSet,
                            vec![make1(AstKind::Match(MatchKind::Range('b', 'z')))],
                        ),
                        make1(AstKind::Match(MatchKind::Char('d'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[b-z]";
                let expect = Ok(make2(
                    AstKind::IncludeSet,
                    vec![make1(AstKind::Match(MatchKind::Range('b', 'z')))],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[bcd]";
                let expect = Ok(make2(
                    AstKind::IncludeSet,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('b'))),
                        make1(AstKind::Match(MatchKind::Char('c'))),
                        make1(AstKind::Match(MatchKind::Char('d'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "a[bc-yz]d";
                let expect = Ok(make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::IncludeSet,
                            vec![
                                make1(AstKind::Match(MatchKind::Char('b'))),
                                make1(AstKind::Match(MatchKind::Range('c', 'y'))),
                                make1(AstKind::Match(MatchKind::Char('z'))),
                            ],
                        ),
                        make1(AstKind::Match(MatchKind::Char('d'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[z-z]";
                let expect = Ok(make2(
                    AstKind::IncludeSet,
                    vec![make1(AstKind::Match(MatchKind::Range('z', 'z')))],
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
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::ExcludeSet,
                            vec![make1(AstKind::Match(MatchKind::Range('b', 'z')))],
                        ),
                        make1(AstKind::Match(MatchKind::Char('d'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[^b-z]";
                let expect = Ok(make2(
                    AstKind::ExcludeSet,
                    vec![make1(AstKind::Match(MatchKind::Range('b', 'z')))],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[^bcd]";
                let expect = Ok(make2(
                    AstKind::ExcludeSet,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('b'))),
                        make1(AstKind::Match(MatchKind::Char('c'))),
                        make1(AstKind::Match(MatchKind::Char('d'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "a[^bc-yz]d";
                let expect = Ok(make2(
                    AstKind::Group,
                    vec![
                        make1(AstKind::Match(MatchKind::Char('a'))),
                        make2(
                            AstKind::ExcludeSet,
                            vec![
                                make1(AstKind::Match(MatchKind::Char('b'))),
                                make1(AstKind::Match(MatchKind::Range('c', 'y'))),
                                make1(AstKind::Match(MatchKind::Char('z'))),
                            ],
                        ),
                        make1(AstKind::Match(MatchKind::Char('d'))),
                    ],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[^z-z]";
                let expect = Ok(make2(
                    AstKind::ExcludeSet,
                    vec![make1(AstKind::Match(MatchKind::Range('z', 'z')))],
                ));

                assert_eq!(run(src), expect);
            }
            {
                let src = "[^z-b]";
                assert_eq!(run(src).is_err(), true);
            }
        }
    }
}
