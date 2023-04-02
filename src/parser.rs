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

pub mod ast;
mod parser;

pub use ast::Ast;
pub use parser::Parser;

#[cfg(test)]
mod tests {
    use super::ast::*;
    use super::parser::*;

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
    mod greedy {
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
    mod non_greedy {
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
