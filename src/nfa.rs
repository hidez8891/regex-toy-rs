use crate::parser::{MatchKind, Parser, PosKind, RepeatKind, SetKind, SyntaxKind, SyntaxNode};
use std::collections::BTreeSet;

pub struct Nfa {
    nodes: Vec<Node>,
}

impl Nfa {
    pub fn new(pattern: &str) -> Result<Nfa, String> {
        let syntax = Parser::new(pattern)?;
        let nfa = Generator::new(&syntax);
        Ok(nfa)
    }

    pub fn is_match<'a>(&self, str: &'a str) -> Option<&'a str> {
        Matcher::is_match(self, str)
    }
}

struct Node {
    nexts: Vec<Edge>,
}

struct Edge {
    action: EdgeAction,
    next_id: usize,
}

enum EdgeAction {
    Asap,
    Match(char),
    MatchAny,
    MatchSOL,
    MatchEOL,
    MatchSet(Vec<MatchSetItem>),
    UnmatchSet(Vec<MatchSetItem>),
}

#[derive(PartialEq, PartialOrd, Eq, Ord)]
enum MatchSetItem {
    Char(char),
    Range(char, char),
}

struct Generator {
    nodes: Vec<Node>,
}

impl Generator {
    #[allow(clippy::new_ret_no_self)]
    fn new(syntax: &SyntaxNode) -> Nfa {
        let mut generator = Generator {
            nodes: vec![
                Node { nexts: vec![] }, // root
                Node { nexts: vec![] }, // submit
            ],
        };
        generator.make(syntax);

        Nfa {
            nodes: generator.nodes,
        }
    }

    /// `make` recursively parses the `syntax` and constructs the NFA.
    fn make(&mut self, syntax: &SyntaxNode) {
        // build the NFA recursively from root and receive the next start-node-ID.
        let node_id = self.make_root(syntax, 1);

        // bind the next node to the first node.
        self.nodes[0].nexts.push(Edge {
            action: EdgeAction::Asap,
            next_id: node_id,
        });
    }

    fn make_root(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        match &syntax.kind {
            SyntaxKind::Group => self.make_group(syntax, dst_id),
            SyntaxKind::Union => self.make_union(syntax, dst_id),
            SyntaxKind::Longest(kind) => match kind {
                RepeatKind::Star => self.make_star(syntax, true, dst_id),
                RepeatKind::Plus => self.make_plus(syntax, true, dst_id),
                RepeatKind::Option => self.make_option(syntax, true, dst_id),
                RepeatKind::Repeat(n) => self.make_repeat(*n, syntax, dst_id),
                RepeatKind::RepeatMin(n) => self.make_repeat_min(*n, syntax, true, dst_id),
                RepeatKind::RepeatRange(a, b) => {
                    self.make_repeat_range(*a, *b, syntax, true, dst_id)
                }
            },
            SyntaxKind::Shortest(kind) => match kind {
                RepeatKind::Star => self.make_star(syntax, false, dst_id),
                RepeatKind::Plus => self.make_plus(syntax, false, dst_id),
                RepeatKind::Option => self.make_option(syntax, false, dst_id),
                RepeatKind::Repeat(n) => self.make_repeat(*n, syntax, dst_id),
                RepeatKind::RepeatMin(n) => self.make_repeat_min(*n, syntax, false, dst_id),
                RepeatKind::RepeatRange(a, b) => {
                    self.make_repeat_range(*a, *b, syntax, false, dst_id)
                }
            },
            SyntaxKind::Match(kind) => match kind {
                MatchKind::Any => self.make_match_any(dst_id),
                MatchKind::Char(c) => self.make_match_char(*c, dst_id),
                MatchKind::Range(_, _) => unreachable!(),
            },
            SyntaxKind::Pos(kind) => match kind {
                PosKind::SOL => self.make_match_sol(dst_id),
                PosKind::EOL => self.make_match_eol(dst_id),
            },
            SyntaxKind::Set(kind) => match kind {
                SetKind::Positive => self.make_positive_set(syntax, dst_id),
                SetKind::Negative => self.make_negative_set(syntax, dst_id),
            },
            SyntaxKind::None => unreachable!(),
        }
    }

    fn make_group(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let mut dst_id = dst_id;
        for child in syntax.children.iter().rev() {
            let match_id = self.make_root(child, dst_id);
            dst_id = match_id;
        }
        dst_id
    }

    fn make_union(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let node_id = self.nodes.len();
        self.nodes.push(Node { nexts: vec![] });

        for child in syntax.children.iter() {
            let match_id = self.make_root(child, dst_id);
            self.nodes[node_id].nexts.push(Edge {
                action: EdgeAction::Asap,
                next_id: match_id,
            });
        }
        node_id
    }

    fn make_star(&mut self, syntax: &SyntaxNode, is_longest: bool, dst_id: usize) -> usize {
        let loop_id = self.nodes.len();
        self.nodes.push(Node { nexts: vec![] });

        if !is_longest {
            self.nodes[loop_id].nexts.push(Edge {
                action: EdgeAction::Asap,
                next_id: dst_id,
            });
        }

        let match_id = self.make_root(&syntax.children[0], loop_id);
        self.nodes[loop_id].nexts.push(Edge {
            action: EdgeAction::Asap,
            next_id: match_id,
        });

        if is_longest {
            self.nodes[loop_id].nexts.push(Edge {
                action: EdgeAction::Asap,
                next_id: dst_id,
            });
        }

        loop_id
    }

    fn make_plus(&mut self, syntax: &SyntaxNode, is_longest: bool, dst_id: usize) -> usize {
        let loop_id = self.nodes.len();
        self.nodes.push(Node { nexts: vec![] });

        if !is_longest {
            self.nodes[loop_id].nexts.push(Edge {
                action: EdgeAction::Asap,
                next_id: dst_id,
            });
        }

        let match_id = self.make_root(&syntax.children[0], loop_id);
        self.nodes[loop_id].nexts.push(Edge {
            action: EdgeAction::Asap,
            next_id: match_id,
        });

        if is_longest {
            self.nodes[loop_id].nexts.push(Edge {
                action: EdgeAction::Asap,
                next_id: dst_id,
            });
        }

        match_id
    }

    fn make_repeat(&mut self, count: u32, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let mut dst_id = dst_id;
        let child = &syntax.children[0];
        for _ in 0..count {
            let match_id = self.make_root(child, dst_id);
            dst_id = match_id;
        }
        dst_id
    }

    fn make_repeat_min(
        &mut self,
        count: u32,
        syntax: &SyntaxNode,
        is_longest: bool,
        dst_id: usize,
    ) -> usize {
        let loop_id = self.make_star(syntax, is_longest, dst_id);
        self.make_repeat(count, syntax, loop_id)
    }

    fn make_repeat_range(
        &mut self,
        min: u32,
        max: u32,
        syntax: &SyntaxNode,
        is_longest: bool,
        dst_id: usize,
    ) -> usize {
        let mut match_id = dst_id;
        let child = &syntax.children[0];
        for _ in min..max {
            let repeat_id = self.make_root(child, match_id);
            self.nodes[repeat_id].nexts.push(Edge {
                action: EdgeAction::Asap,
                next_id: dst_id,
            });

            match_id = repeat_id;
        }

        self.make_repeat(min, syntax, match_id)
    }

    fn make_option(&mut self, syntax: &SyntaxNode, is_longest: bool, dst_id: usize) -> usize {
        let match_id = self.make_root(&syntax.children[0], dst_id);
        self.nodes[match_id].nexts.push(Edge {
            action: EdgeAction::Asap,
            next_id: dst_id,
        });
        match_id
    }

    fn make_match_any(&mut self, dst_id: usize) -> usize {
        let node_id = self.nodes.len();
        self.nodes.push(Node {
            nexts: vec![Edge {
                action: EdgeAction::MatchAny,
                next_id: dst_id,
            }],
        });
        node_id
    }

    fn make_match_sol(&mut self, dst_id: usize) -> usize {
        let node_id = self.nodes.len();
        self.nodes.push(Node {
            nexts: vec![Edge {
                action: EdgeAction::MatchSOL,
                next_id: dst_id,
            }],
        });
        node_id
    }

    fn make_match_eol(&mut self, dst_id: usize) -> usize {
        let node_id = self.nodes.len();
        self.nodes.push(Node {
            nexts: vec![Edge {
                action: EdgeAction::MatchEOL,
                next_id: dst_id,
            }],
        });
        node_id
    }

    fn make_match_char(&mut self, c: char, dst_id: usize) -> usize {
        let node_id = self.nodes.len();
        self.nodes.push(Node {
            nexts: vec![Edge {
                action: EdgeAction::Match(c),
                next_id: dst_id,
            }],
        });
        node_id
    }

    fn make_positive_set(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let set = self.make_set_items(syntax).into_iter().collect::<Vec<_>>();

        let node_id = self.nodes.len();
        self.nodes.push(Node {
            nexts: vec![Edge {
                action: EdgeAction::MatchSet(set),
                next_id: dst_id,
            }],
        });
        node_id
    }

    fn make_negative_set(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let set = self.make_set_items(syntax).into_iter().collect::<Vec<_>>();

        let node_id = self.nodes.len();
        self.nodes.push(Node {
            nexts: vec![Edge {
                action: EdgeAction::UnmatchSet(set),
                next_id: dst_id,
            }],
        });
        node_id
    }

    #[allow(clippy::only_used_in_recursion)]
    fn make_set_items(&self, syntax: &SyntaxNode) -> BTreeSet<MatchSetItem> {
        let mut set = BTreeSet::new();
        for child in syntax.children.iter() {
            match &child.kind {
                SyntaxKind::Group => {
                    let res = self.make_set_items(child);
                    set.extend(res.into_iter());
                }
                SyntaxKind::Match(kind) => match kind {
                    MatchKind::Char(c) => {
                        set.insert(MatchSetItem::Char(*c));
                    }
                    MatchKind::Range(a, b) => {
                        set.insert(MatchSetItem::Range(*a, *b));
                    }
                    MatchKind::Any => unreachable!(),
                },
                _ => unreachable!(),
            };
        }
        set
    }
}

struct Matcher<'a, 'b> {
    nfa: &'a Nfa,
    str: &'b str,
    start_index: usize,
}

impl<'a, 'b> Matcher<'a, 'b> {
    fn is_match(nfa: &'a Nfa, str: &'b str) -> Option<&'b str> {
        let mut matcher = Matcher {
            nfa,
            str,
            start_index: 0,
        };

        for i in 0..str.len() {
            matcher.start_index = i;

            let result = matcher.is_match_impl(i, 0);
            if result.is_some() {
                return result;
            }
        }
        None
    }

    fn is_match_impl(&self, index: usize, node_id: usize) -> Option<&'b str> {
        if node_id == 1 {
            return Some(&self.str[self.start_index..index]);
        }

        let node = &self.nfa.nodes[node_id];
        for edge in node.nexts.iter() {
            #[rustfmt::skip]
            let result = match &edge.action {
                EdgeAction::Asap =>
                    self.is_match_impl(index, edge.next_id),
                EdgeAction::Match(t) =>
                    self.str.chars().nth(index)
                        .filter(|c| *c == *t)
                        .and_then(|_|
                            self.is_match_impl(index + 1, edge.next_id)
                        ),
                EdgeAction::MatchAny =>
                    self.str.chars().nth(index)
                        .and_then(|_|
                            self.is_match_impl(index + 1, edge.next_id),
                        ),
                EdgeAction::MatchSOL =>
                    Some(index).filter(|p| *p == 0)
                        .and_then(|_|
                            self.is_match_impl(index, edge.next_id)
                        ),
                EdgeAction::MatchEOL =>
                    Some(index).filter(|p| *p == self.str.len())
                        .and_then(|_|
                            self.is_match_impl(index, edge.next_id)
                        ),
                EdgeAction::MatchSet(set) =>
                    self.str.chars().nth(index)
                        .filter(|c|
                            set.iter().any(|m| match m {
                                MatchSetItem::Char(t) => *t == *c,
                                MatchSetItem::Range(a, b) => *a <= *c && *c <= *b,
                            })
                        )
                        .and_then(|_|
                            self.is_match_impl(index + 1, edge.next_id)
                        ),
                EdgeAction::UnmatchSet(set) =>
                    self.str.chars().nth(index)
                        .filter(|c|
                            set.iter().all(|m| match m {
                                MatchSetItem::Char(t) => *t != *c,
                                MatchSetItem::Range(a, b) => *c < *a || *b < *c,
                            })
                        )
                        .and_then(|_|
                            self.is_match_impl(index + 1, edge.next_id)
                        ),
            };

            if result.is_some() {
                return result;
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(pattern: &str) -> Nfa {
        Nfa::new(pattern).unwrap()
    }

    #[cfg(test)]
    mod basic_match {
        use super::*;

        #[test]
        fn match_char() {
            let src = "abc";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abc"), Some("abc"));
            assert_eq!(nfa.is_match("ab"), None);
            assert_eq!(nfa.is_match("abcd"), Some("abc"));
            assert_eq!(nfa.is_match("zabc"), Some("abc"));
        }

        #[test]
        fn match_metachar() {
            let src = r"a\+c";
            let nfa = run(src);

            assert_eq!(nfa.is_match("a+c"), Some("a+c"));
            assert_eq!(nfa.is_match("aac"), None);
            assert_eq!(nfa.is_match("ac"), None);
            assert_eq!(nfa.is_match("a+cz"), Some("a+c"));
            assert_eq!(nfa.is_match("za+c"), Some("a+c"));
        }

        #[test]
        fn match_any() {
            {
                let src = "a.c";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abc"), Some("abc"));
                assert_eq!(nfa.is_match("adc"), Some("adc"));
                assert_eq!(nfa.is_match("ac"), None);
                assert_eq!(nfa.is_match("abbc"), None);
                assert_eq!(nfa.is_match("zabc"), Some("abc"));
                assert_eq!(nfa.is_match("abcz"), Some("abc"));
            }
            {
                let src = "a.";
                let nfa = run(src);

                assert_eq!(nfa.is_match("ab"), Some("ab"));
                assert_eq!(nfa.is_match("ad"), Some("ad"));
                assert_eq!(nfa.is_match("a"), None);
                assert_eq!(nfa.is_match("abz"), Some("ab"));
                assert_eq!(nfa.is_match("zab"), Some("ab"));
            }
        }

        #[test]
        fn match_sol() {
            {
                let src = "^abc";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abc"), Some("abc"));
                assert_eq!(nfa.is_match("zabc"), None);
                assert_eq!(nfa.is_match("abcz"), Some("abc"));
            }
        }

        #[test]
        fn match_eol() {
            {
                let src = "abc$";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abc"), Some("abc"));
                assert_eq!(nfa.is_match("zabc"), Some("abc"));
                assert_eq!(nfa.is_match("abcz"), None);
            }
        }
    }

    #[test]
    fn group() {
        {
            let src = "a(bc)d";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abcd"), Some("abcd"));
            assert_eq!(nfa.is_match("abc"), None);
            assert_eq!(nfa.is_match("ad"), None);
            assert_eq!(nfa.is_match("zabcd"), Some("abcd"));
            assert_eq!(nfa.is_match("abcdz"), Some("abcd"));
        }
        {
            let src = "a(bc)";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abc"), Some("abc"));
            assert_eq!(nfa.is_match("a"), None);
            assert_eq!(nfa.is_match("zabc"), Some("abc"));
            assert_eq!(nfa.is_match("abcd"), Some("abc"));
        }
    }

    #[test]
    fn union() {
        let src = "abc|def|ghi";
        let nfa = run(src);

        assert_eq!(nfa.is_match("abc"), Some("abc"));
        assert_eq!(nfa.is_match("def"), Some("def"));
        assert_eq!(nfa.is_match("ghi"), Some("ghi"));
        assert_eq!(nfa.is_match("adg"), None);
        assert_eq!(nfa.is_match("ab"), None);
        assert_eq!(nfa.is_match("zabc"), Some("abc"));
        assert_eq!(nfa.is_match("defz"), Some("def"));
    }

    #[cfg(test)]
    mod longest {
        use super::*;

        #[test]
        fn star() {
            {
                let src = "ab*c";
                let nfa = run(src);

                assert_eq!(nfa.is_match("ac"), Some("ac"));
                assert_eq!(nfa.is_match("abc"), Some("abc"));
                assert_eq!(nfa.is_match("abbc"), Some("abbc"));
                assert_eq!(nfa.is_match("abbbc"), Some("abbbc"));
                assert_eq!(nfa.is_match("az"), None);
                assert_eq!(nfa.is_match("zac"), Some("ac"));
                assert_eq!(nfa.is_match("acz"), Some("ac"));
            }
            {
                let src = "ab*";
                let nfa = run(src);

                assert_eq!(nfa.is_match("a"), Some("a"));
                assert_eq!(nfa.is_match("ab"), Some("ab"));
                assert_eq!(nfa.is_match("abb"), Some("abb"));
                assert_eq!(nfa.is_match("abbb"), Some("abbb"));
                assert_eq!(nfa.is_match("b"), None);
                assert_eq!(nfa.is_match("za"), Some("a"));
                assert_eq!(nfa.is_match("az"), Some("a"));
            }
            {
                let src = "ab*b*";
                let nfa = run(src);

                assert_eq!(nfa.is_match("a"), Some("a"));
                assert_eq!(nfa.is_match("ab"), Some("ab"));
                assert_eq!(nfa.is_match("abb"), Some("abb"));
                assert_eq!(nfa.is_match("abbb"), Some("abbb"));
                assert_eq!(nfa.is_match("b"), None);
                assert_eq!(nfa.is_match("za"), Some("a"));
                assert_eq!(nfa.is_match("az"), Some("a"));
            }
            {
                let src = "a.*b";
                let nfa = run(src);

                assert_eq!(nfa.is_match("ab"), Some("ab"));
                assert_eq!(nfa.is_match("axb"), Some("axb"));
                assert_eq!(nfa.is_match("axbaxb"), Some("axbaxb"));
                #[rustfmt::skip]
            assert_eq!(nfa.is_match("axaxbxb"), Some("axaxbxb"));
                assert_eq!(nfa.is_match("baxb"), Some("axb"));
                assert_eq!(nfa.is_match("axbz"), Some("axb"));
            }
        }

        #[test]
        fn plus() {
            {
                let src = "ab+c";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abc"), Some("abc"));
                assert_eq!(nfa.is_match("abbc"), Some("abbc"));
                assert_eq!(nfa.is_match("abbbc"), Some("abbbc"));
                assert_eq!(nfa.is_match("ac"), None);
                assert_eq!(nfa.is_match("zabc"), Some("abc"));
                assert_eq!(nfa.is_match("abcz"), Some("abc"));
            }
            {
                let src = "ab+";
                let nfa = run(src);

                assert_eq!(nfa.is_match("ab"), Some("ab"));
                assert_eq!(nfa.is_match("abb"), Some("abb"));
                assert_eq!(nfa.is_match("abbb"), Some("abbb"));
                assert_eq!(nfa.is_match("a"), None);
                assert_eq!(nfa.is_match("zab"), Some("ab"));
                assert_eq!(nfa.is_match("abz"), Some("ab"));
            }
            {
                let src = "ab+b+";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abb"), Some("abb"));
                assert_eq!(nfa.is_match("abbb"), Some("abbb"));
                assert_eq!(nfa.is_match("abbbb"), Some("abbbb"));
                assert_eq!(nfa.is_match("a"), None);
                assert_eq!(nfa.is_match("ab"), None);
                assert_eq!(nfa.is_match("zabb"), Some("abb"));
                assert_eq!(nfa.is_match("abbz"), Some("abb"));
            }
            {
                let src = "a.+b";
                let nfa = run(src);

                assert_eq!(nfa.is_match("ab"), None);
                assert_eq!(nfa.is_match("axb"), Some("axb"));
                assert_eq!(nfa.is_match("axbaxb"), Some("axbaxb"));
                assert_eq!(nfa.is_match("axaxbxb"), Some("axaxbxb"));
                assert_eq!(nfa.is_match("baxb"), Some("axb"));
                assert_eq!(nfa.is_match("axbz"), Some("axb"));
            }
        }

        #[test]
        fn option() {
            {
                let src = "ab?c";
                let nfa = run(src);

                assert_eq!(nfa.is_match("ac"), Some("ac"));
                assert_eq!(nfa.is_match("abc"), Some("abc"));
                assert_eq!(nfa.is_match("a"), None);
                assert_eq!(nfa.is_match("zac"), Some("ac"));
                assert_eq!(nfa.is_match("acz"), Some("ac"));
            }
            {
                let src = "ab?";
                let nfa = run(src);

                assert_eq!(nfa.is_match("a"), Some("a"));
                assert_eq!(nfa.is_match("ab"), Some("ab"));
                assert_eq!(nfa.is_match("b"), None);
                assert_eq!(nfa.is_match("za"), Some("a"));
                assert_eq!(nfa.is_match("az"), Some("a"));
            }
        }

        #[test]
        fn repeat() {
            {
                let src = "a{3}";
                let nfa = run(src);

                assert_eq!(nfa.is_match("aaa"), Some("aaa"));
                assert_eq!(nfa.is_match("aaaaa"), Some("aaa"));
                assert_eq!(nfa.is_match("aa"), None);
                assert_eq!(nfa.is_match("zaaa"), Some("aaa"));
                assert_eq!(nfa.is_match("aaaz"), Some("aaa"));
            }
            {
                let src = "abc{3}";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abccc"), Some("abccc"));
                assert_eq!(nfa.is_match("abccccc"), Some("abccc"));
                assert_eq!(nfa.is_match("abc"), None);
                assert_eq!(nfa.is_match("zabccc"), Some("abccc"));
                assert_eq!(nfa.is_match("abcccz"), Some("abccc"));
            }
            {
                let src = "(abc){3}";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abcabcabc"), Some("abcabcabc"));
                assert_eq!(nfa.is_match("abcabc"), None);
                assert_eq!(nfa.is_match("zabcabcabc"), Some("abcabcabc"));
                assert_eq!(nfa.is_match("abcabcabcz"), Some("abcabcabc"));
            }
        }

        #[test]
        fn repeat_min() {
            {
                let src = "a{2,}";
                let nfa = run(src);

                assert_eq!(nfa.is_match("aa"), Some("aa"));
                assert_eq!(nfa.is_match("aaa"), Some("aaa"));
                assert_eq!(nfa.is_match("a"), None);
                assert_eq!(nfa.is_match("zaaa"), Some("aaa"));
                assert_eq!(nfa.is_match("aaaz"), Some("aaa"));
            }
            {
                let src = "abc{2,}";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abcc"), Some("abcc"));
                assert_eq!(nfa.is_match("abccc"), Some("abccc"));
                assert_eq!(nfa.is_match("abc"), None);
                assert_eq!(nfa.is_match("zabcc"), Some("abcc"));
                assert_eq!(nfa.is_match("abccz"), Some("abcc"));
            }
            {
                let src = "(abc){2,}";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abcabc"), Some("abcabc"));
                assert_eq!(nfa.is_match("abcabcabc"), Some("abcabcabc"));
                assert_eq!(nfa.is_match("abc"), None);
                assert_eq!(nfa.is_match("zabcabc"), Some("abcabc"));
                assert_eq!(nfa.is_match("abcabcz"), Some("abcabc"));
            }
        }

        #[test]
        fn repeat_range() {
            {
                let src = "a{2,3}";
                let nfa = run(src);

                assert_eq!(nfa.is_match("aa"), Some("aa"));
                assert_eq!(nfa.is_match("aaa"), Some("aaa"));
                assert_eq!(nfa.is_match("aaaa"), Some("aaa"));
                assert_eq!(nfa.is_match("a"), None);
                assert_eq!(nfa.is_match("zaa"), Some("aa"));
                assert_eq!(nfa.is_match("aaz"), Some("aa"));
            }
            {
                let src = "abc{2,3}";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abcc"), Some("abcc"));
                assert_eq!(nfa.is_match("abccc"), Some("abccc"));
                assert_eq!(nfa.is_match("abcccc"), Some("abccc"));
                assert_eq!(nfa.is_match("abc"), None);
                assert_eq!(nfa.is_match("zabcc"), Some("abcc"));
                assert_eq!(nfa.is_match("abccz"), Some("abcc"));
            }
            {
                let src = "(abc){2,3}";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abcabc"), Some("abcabc"));
                assert_eq!(nfa.is_match("abcabcabc"), Some("abcabcabc"));
                assert_eq!(nfa.is_match("abcabcabcabc"), Some("abcabcabc"));
                assert_eq!(nfa.is_match("abc"), None);
                assert_eq!(nfa.is_match("zabcabc"), Some("abcabc"));
                assert_eq!(nfa.is_match("abcabcz"), Some("abcabc"));
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
                let nfa = run(src);

                assert_eq!(nfa.is_match("ac"), Some("ac"));
                assert_eq!(nfa.is_match("abc"), Some("abc"));
                assert_eq!(nfa.is_match("abbc"), Some("abbc"));
                assert_eq!(nfa.is_match("abbbc"), Some("abbbc"));
                assert_eq!(nfa.is_match("az"), None);
                assert_eq!(nfa.is_match("zac"), Some("ac"));
                assert_eq!(nfa.is_match("acz"), Some("ac"));
            }
            {
                let src = "ab*?";
                let nfa = run(src);

                assert_eq!(nfa.is_match("a"), Some("a"));
                assert_eq!(nfa.is_match("ab"), Some("a"));
                assert_eq!(nfa.is_match("abb"), Some("a"));
                assert_eq!(nfa.is_match("abbb"), Some("a"));
                assert_eq!(nfa.is_match("b"), None);
                assert_eq!(nfa.is_match("za"), Some("a"));
                assert_eq!(nfa.is_match("az"), Some("a"));
            }
            {
                let src = "ab*?b*?";
                let nfa = run(src);

                assert_eq!(nfa.is_match("a"), Some("a"));
                assert_eq!(nfa.is_match("ab"), Some("a"));
                assert_eq!(nfa.is_match("abb"), Some("a"));
                assert_eq!(nfa.is_match("abbb"), Some("a"));
                assert_eq!(nfa.is_match("b"), None);
                assert_eq!(nfa.is_match("za"), Some("a"));
                assert_eq!(nfa.is_match("az"), Some("a"));
            }
            {
                let src = "a.*?b";
                let nfa = run(src);

                assert_eq!(nfa.is_match("ab"), Some("ab"));
                assert_eq!(nfa.is_match("axb"), Some("axb"));
                assert_eq!(nfa.is_match("axbaxb"), Some("axb"));
                #[rustfmt::skip]
            assert_eq!(nfa.is_match("axaxbxb"), Some("axaxb"));
                assert_eq!(nfa.is_match("baxb"), Some("axb"));
                assert_eq!(nfa.is_match("axbz"), Some("axb"));
            }
        }

        #[test]
        fn plus() {
            {
                let src = "ab+?c";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abc"), Some("abc"));
                assert_eq!(nfa.is_match("abbc"), Some("abbc"));
                assert_eq!(nfa.is_match("abbbc"), Some("abbbc"));
                assert_eq!(nfa.is_match("ac"), None);
                assert_eq!(nfa.is_match("zabc"), Some("abc"));
                assert_eq!(nfa.is_match("abcz"), Some("abc"));
            }
            {
                let src = "ab+?";
                let nfa = run(src);

                assert_eq!(nfa.is_match("ab"), Some("ab"));
                assert_eq!(nfa.is_match("abb"), Some("ab"));
                assert_eq!(nfa.is_match("abbb"), Some("ab"));
                assert_eq!(nfa.is_match("a"), None);
                assert_eq!(nfa.is_match("zab"), Some("ab"));
                assert_eq!(nfa.is_match("abz"), Some("ab"));
            }
            {
                let src = "ab+?b+?";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abb"), Some("abb"));
                assert_eq!(nfa.is_match("abbb"), Some("abb"));
                assert_eq!(nfa.is_match("abbbb"), Some("abb"));
                assert_eq!(nfa.is_match("a"), None);
                assert_eq!(nfa.is_match("ab"), None);
                assert_eq!(nfa.is_match("zabb"), Some("abb"));
                assert_eq!(nfa.is_match("abbz"), Some("abb"));
            }
            {
                let src = "a.+?b";
                let nfa = run(src);

                assert_eq!(nfa.is_match("ab"), None);
                assert_eq!(nfa.is_match("axb"), Some("axb"));
                assert_eq!(nfa.is_match("axbaxb"), Some("axb"));
                assert_eq!(nfa.is_match("axaxbxb"), Some("axaxb"));
                assert_eq!(nfa.is_match("baxb"), Some("axb"));
                assert_eq!(nfa.is_match("axbz"), Some("axb"));
            }
        }
    }

    #[test]
    fn positive() {
        {
            let src = "a[b-z]d";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abd"), Some("abd"));
            assert_eq!(nfa.is_match("azd"), Some("azd"));
            assert_eq!(nfa.is_match("axd"), Some("axd"));
            assert_eq!(nfa.is_match("ad"), None);
            assert_eq!(nfa.is_match("aad"), None);
            assert_eq!(nfa.is_match("zabd"), Some("abd"));
            assert_eq!(nfa.is_match("abdz"), Some("abd"));
        }
        {
            let src = "[b-z]";
            let nfa = run(src);

            assert_eq!(nfa.is_match("b"), Some("b"));
            assert_eq!(nfa.is_match("z"), Some("z"));
            assert_eq!(nfa.is_match("x"), Some("x"));
            assert_eq!(nfa.is_match("a"), None);
            assert_eq!(nfa.is_match("ab"), Some("b"));
            assert_eq!(nfa.is_match("bz"), Some("b"));
        }
        {
            let src = "[bcd]";
            let nfa = run(src);

            assert_eq!(nfa.is_match("b"), Some("b"));
            assert_eq!(nfa.is_match("c"), Some("c"));
            assert_eq!(nfa.is_match("d"), Some("d"));
            assert_eq!(nfa.is_match("a"), None);
            assert_eq!(nfa.is_match("e"), None);
            assert_eq!(nfa.is_match("ab"), Some("b"));
            assert_eq!(nfa.is_match("bz"), Some("b"));
        }
        {
            let src = "a[bc-yz]d";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abd"), Some("abd"));
            assert_eq!(nfa.is_match("azd"), Some("azd"));
            assert_eq!(nfa.is_match("acd"), Some("acd"));
            assert_eq!(nfa.is_match("ayd"), Some("ayd"));
            assert_eq!(nfa.is_match("axd"), Some("axd"));
            assert_eq!(nfa.is_match("aad"), None);
            assert_eq!(nfa.is_match("ad"), None);
            assert_eq!(nfa.is_match("zabd"), Some("abd"));
            assert_eq!(nfa.is_match("abdz"), Some("abd"));
        }
        {
            let src = "[z-z]";
            let nfa = run(src);

            assert_eq!(nfa.is_match("z"), Some("z"));
            assert_eq!(nfa.is_match("a"), None);
            assert_eq!(nfa.is_match("az"), Some("z"));
            assert_eq!(nfa.is_match("za"), Some("z"));
        }
    }

    #[cfg(test)]
    mod set {
        use super::*;

        #[test]
        fn negative() {
            {
                let src = "a[^b-z]d";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abd"), None);
                assert_eq!(nfa.is_match("azd"), None);
                assert_eq!(nfa.is_match("axd"), None);
                assert_eq!(nfa.is_match("aad"), Some("aad"));
                assert_eq!(nfa.is_match("ad"), None);
                assert_eq!(nfa.is_match("zaad"), Some("aad"));
                assert_eq!(nfa.is_match("aadz"), Some("aad"));
            }
            {
                let src = "[^b-z]";
                let nfa = run(src);

                assert_eq!(nfa.is_match("b"), None);
                assert_eq!(nfa.is_match("z"), None);
                assert_eq!(nfa.is_match("x"), None);
                assert_eq!(nfa.is_match("a"), Some("a"));
                assert_eq!(nfa.is_match("za"), Some("a"));
                assert_eq!(nfa.is_match("az"), Some("a"));
            }
            {
                let src = "[^bcd]";
                let nfa = run(src);

                assert_eq!(nfa.is_match("b"), None);
                assert_eq!(nfa.is_match("c"), None);
                assert_eq!(nfa.is_match("d"), None);
                assert_eq!(nfa.is_match("a"), Some("a"));
                assert_eq!(nfa.is_match("e"), Some("e"));
                assert_eq!(nfa.is_match("ba"), Some("a"));
                assert_eq!(nfa.is_match("ab"), Some("a"));
            }
            {
                let src = "a[^bc-yz]d";
                let nfa = run(src);

                assert_eq!(nfa.is_match("abd"), None);
                assert_eq!(nfa.is_match("azd"), None);
                assert_eq!(nfa.is_match("acd"), None);
                assert_eq!(nfa.is_match("ayd"), None);
                assert_eq!(nfa.is_match("axd"), None);
                assert_eq!(nfa.is_match("aad"), Some("aad"));
                assert_eq!(nfa.is_match("ad"), None);
                assert_eq!(nfa.is_match("zaad"), Some("aad"));
                assert_eq!(nfa.is_match("aadz"), Some("aad"));
            }
            {
                let src = "[^z-z]";
                let nfa = run(src);

                assert_eq!(nfa.is_match("z"), None);
                assert_eq!(nfa.is_match("a"), Some("a"));
                assert_eq!(nfa.is_match("za"), Some("a"));
                assert_eq!(nfa.is_match("az"), Some("a"));
            }
        }
    }

    #[test]
    fn pattern001() {
        {
            let src = r"[a-zA-Z0-9_\.\+\-]+@[a-zA-Z0-9_\.]+[a-zA-Z]+";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abc@example.com"), Some("abc@example.com"));
            assert_eq!(
                nfa.is_match("abc+123@me.example.com"),
                Some("abc+123@me.example.com")
            );
            assert_eq!(nfa.is_match("abc@example"), Some("abc@example"));
            assert_eq!(nfa.is_match("abc@example.123"), Some("abc@example"));
            assert_eq!(nfa.is_match("abc@def@example.com"), Some("abc@def"));
        }
        {
            let src = r"^[a-zA-Z0-9_\.\+\-]+@[a-zA-Z0-9_\.]+[a-zA-Z]+$";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abc@example.com"), Some("abc@example.com"));
            assert_eq!(
                nfa.is_match("abc+123@me.example.com"),
                Some("abc+123@me.example.com")
            );
            assert_eq!(nfa.is_match("abc@example"), Some("abc@example"));
            assert_eq!(nfa.is_match("abc@example.123"), None);
            assert_eq!(nfa.is_match("abc@def@example.com"), None);
        }
    }
}
