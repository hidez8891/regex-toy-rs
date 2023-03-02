use crate::parser::{Parser, SyntaxKind, SyntaxNode};
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

    pub fn is_match(&self, str: &str) -> Option<String> {
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
        match syntax.kind {
            SyntaxKind::Group => self.make_group(syntax, dst_id),
            SyntaxKind::Union => self.make_union(syntax, dst_id),
            SyntaxKind::ManyStar => self.make_many_star(syntax, dst_id),
            SyntaxKind::ManyPlus => self.make_many_plus(syntax, dst_id),
            SyntaxKind::Option => self.make_option(syntax, dst_id),
            SyntaxKind::MatchAny => self.make_match_any(dst_id),
            SyntaxKind::MatchSOL => self.make_match_sol(dst_id),
            SyntaxKind::MatchEOL => self.make_match_eol(dst_id),
            SyntaxKind::Match(c) => self.make_match_char(c, dst_id),
            SyntaxKind::PositiveSet => self.make_positive_set(syntax, dst_id),
            SyntaxKind::NegativeSet => self.make_negative_set(syntax, dst_id),
            _ => unreachable!(),
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

    fn make_many_star(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let loop_id = self.nodes.len();
        self.nodes.push(Node { nexts: vec![] });

        let match_id = self.make_root(&syntax.children[0], loop_id);
        self.nodes[loop_id].nexts.push(Edge {
            action: EdgeAction::Asap,
            next_id: match_id,
        });

        self.nodes[loop_id].nexts.push(Edge {
            action: EdgeAction::Asap,
            next_id: dst_id,
        });
        loop_id
    }

    fn make_many_plus(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let loop_id = self.nodes.len();
        self.nodes.push(Node { nexts: vec![] });

        let match_id = self.make_root(&syntax.children[0], loop_id);
        self.nodes[loop_id].nexts.push(Edge {
            action: EdgeAction::Asap,
            next_id: match_id,
        });

        self.nodes[loop_id].nexts.push(Edge {
            action: EdgeAction::Asap,
            next_id: dst_id,
        });
        match_id
    }

    fn make_option(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
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

    fn make_set_items(&mut self, syntax: &SyntaxNode) -> BTreeSet<MatchSetItem> {
        let mut set = BTreeSet::new();
        for child in syntax.children.iter() {
            match child.kind {
                SyntaxKind::Group => {
                    let res = self.make_set_items(child);
                    set.extend(res.into_iter());
                }
                SyntaxKind::Match(c) => {
                    set.insert(MatchSetItem::Char(c));
                }
                SyntaxKind::MatchRange(a, b) => {
                    set.insert(MatchSetItem::Range(a, b));
                }
                _ => unreachable!(),
            };
        }
        set
    }
}

struct Matcher<'a> {
    nfa: &'a Nfa,
    str: String,
    start_index: usize,
}

impl<'a> Matcher<'a> {
    fn is_match(nfa: &'a Nfa, str: &str) -> Option<String> {
        let mut matcher = Matcher {
            nfa,
            str: str.to_owned(),
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

    fn is_match_impl(&self, index: usize, node_id: usize) -> Option<String> {
        if node_id == 1 {
            return Some(self.str[self.start_index..index].to_string());
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

    #[test]
    fn match_char() {
        let src = "abc";
        let nfa = run(src);

        assert_eq!(nfa.is_match("abc"), Some("abc".to_owned()));
        assert_eq!(nfa.is_match("ab"), None);
        assert_eq!(nfa.is_match("abcd"), Some("abc".to_owned()));
        assert_eq!(nfa.is_match("zabc"), Some("abc".to_owned()));
    }

    #[test]
    fn match_metachar() {
        let src = r"a\+c";
        let nfa = run(src);

        assert_eq!(nfa.is_match("a+c"), Some("a+c".to_owned()));
        assert_eq!(nfa.is_match("aac"), None);
        assert_eq!(nfa.is_match("ac"), None);
        assert_eq!(nfa.is_match("a+cz"), Some("a+c".to_owned()));
        assert_eq!(nfa.is_match("za+c"), Some("a+c".to_owned()));
    }

    #[test]
    fn match_any() {
        {
            let src = "a.c";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abc"), Some("abc".to_owned()));
            assert_eq!(nfa.is_match("adc"), Some("adc".to_owned()));
            assert_eq!(nfa.is_match("ac"), None);
            assert_eq!(nfa.is_match("abbc"), None);
            assert_eq!(nfa.is_match("zabc"), Some("abc".to_owned()));
            assert_eq!(nfa.is_match("abcz"), Some("abc".to_owned()));
        }
        {
            let src = "a.";
            let nfa = run(src);

            assert_eq!(nfa.is_match("ab"), Some("ab".to_owned()));
            assert_eq!(nfa.is_match("ad"), Some("ad".to_owned()));
            assert_eq!(nfa.is_match("a"), None);
            assert_eq!(nfa.is_match("abz"), Some("ab".to_owned()));
            assert_eq!(nfa.is_match("zab"), Some("ab".to_owned()));
        }
    }

    #[test]
    fn match_sol() {
        {
            let src = "^abc";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abc"), Some("abc".to_owned()));
            assert_eq!(nfa.is_match("zabc"), None);
            assert_eq!(nfa.is_match("abcz"), Some("abc".to_owned()));
        }
    }

    #[test]
    fn match_eol() {
        {
            let src = "abc$";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abc"), Some("abc".to_owned()));
            assert_eq!(nfa.is_match("zabc"), Some("abc".to_owned()));
            assert_eq!(nfa.is_match("abcz"), None);
        }
    }

    #[test]
    fn group() {
        {
            let src = "a(bc)d";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abcd"), Some("abcd".to_owned()));
            assert_eq!(nfa.is_match("abc"), None);
            assert_eq!(nfa.is_match("ad"), None);
            assert_eq!(nfa.is_match("zabcd"), Some("abcd".to_owned()));
            assert_eq!(nfa.is_match("abcdz"), Some("abcd".to_owned()));
        }
        {
            let src = "a(bc)";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abc"), Some("abc".to_owned()));
            assert_eq!(nfa.is_match("a"), None);
            assert_eq!(nfa.is_match("zabc"), Some("abc".to_owned()));
            assert_eq!(nfa.is_match("abcd"), Some("abc".to_owned()));
        }
    }

    #[test]
    fn union() {
        let src = "abc|def|ghi";
        let nfa = run(src);

        assert_eq!(nfa.is_match("abc"), Some("abc".to_owned()));
        assert_eq!(nfa.is_match("def"), Some("def".to_owned()));
        assert_eq!(nfa.is_match("ghi"), Some("ghi".to_owned()));
        assert_eq!(nfa.is_match("adg"), None);
        assert_eq!(nfa.is_match("ab"), None);
        assert_eq!(nfa.is_match("zabc"), Some("abc".to_owned()));
        assert_eq!(nfa.is_match("defz"), Some("def".to_owned()));
    }

    #[test]
    fn many_star() {
        {
            let src = "ab*c";
            let nfa = run(src);

            assert_eq!(nfa.is_match("ac"), Some("ac".to_owned()));
            assert_eq!(nfa.is_match("abc"), Some("abc".to_owned()));
            assert_eq!(nfa.is_match("abbc"), Some("abbc".to_owned()));
            assert_eq!(nfa.is_match("abbbc"), Some("abbbc".to_owned()));
            assert_eq!(nfa.is_match("az"), None);
            assert_eq!(nfa.is_match("zac"), Some("ac".to_owned()));
            assert_eq!(nfa.is_match("acz"), Some("ac".to_owned()));
        }
        {
            let src = "ab*";
            let nfa = run(src);

            assert_eq!(nfa.is_match("a"), Some("a".to_owned()));
            assert_eq!(nfa.is_match("ab"), Some("ab".to_owned()));
            assert_eq!(nfa.is_match("abb"), Some("abb".to_owned()));
            assert_eq!(nfa.is_match("abbb"), Some("abbb".to_owned()));
            assert_eq!(nfa.is_match("b"), None);
            assert_eq!(nfa.is_match("za"), Some("a".to_owned()));
            assert_eq!(nfa.is_match("az"), Some("a".to_owned()));
        }
        {
            let src = "ab*b*";
            let nfa = run(src);

            assert_eq!(nfa.is_match("a"), Some("a".to_owned()));
            assert_eq!(nfa.is_match("ab"), Some("ab".to_owned()));
            assert_eq!(nfa.is_match("abb"), Some("abb".to_owned()));
            assert_eq!(nfa.is_match("abbb"), Some("abbb".to_owned()));
            assert_eq!(nfa.is_match("b"), None);
            assert_eq!(nfa.is_match("za"), Some("a".to_owned()));
            assert_eq!(nfa.is_match("az"), Some("a".to_owned()));
        }
        {
            let src = "a.*b";
            let nfa = run(src);

            assert_eq!(nfa.is_match("ab"), Some("ab".to_owned()));
            assert_eq!(nfa.is_match("axb"), Some("axb".to_owned()));
            assert_eq!(nfa.is_match("axbaxb"), Some("axbaxb".to_owned()));
            #[rustfmt::skip]
            assert_eq!(nfa.is_match("axaxbxb"), Some("axaxbxb".to_owned()));
            assert_eq!(nfa.is_match("baxb"), Some("axb".to_owned()));
            assert_eq!(nfa.is_match("axbz"), Some("axb".to_owned()));
        }
    }

    #[test]
    fn many_plus() {
        {
            let src = "ab+c";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abc"), Some("abc".to_owned()));
            assert_eq!(nfa.is_match("abbc"), Some("abbc".to_owned()));
            assert_eq!(nfa.is_match("abbbc"), Some("abbbc".to_owned()));
            assert_eq!(nfa.is_match("ac"), None);
            assert_eq!(nfa.is_match("zabc"), Some("abc".to_owned()));
            assert_eq!(nfa.is_match("abcz"), Some("abc".to_owned()));
        }
        {
            let src = "ab+";
            let nfa = run(src);

            assert_eq!(nfa.is_match("ab"), Some("ab".to_owned()));
            assert_eq!(nfa.is_match("abb"), Some("abb".to_owned()));
            assert_eq!(nfa.is_match("abbb"), Some("abbb".to_owned()));
            assert_eq!(nfa.is_match("a"), None);
            assert_eq!(nfa.is_match("zab"), Some("ab".to_owned()));
            assert_eq!(nfa.is_match("abz"), Some("ab".to_owned()));
        }
        {
            let src = "ab+b+";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abb"), Some("abb".to_owned()));
            assert_eq!(nfa.is_match("abbb"), Some("abbb".to_owned()));
            assert_eq!(nfa.is_match("abbbb"), Some("abbbb".to_owned()));
            assert_eq!(nfa.is_match("a"), None);
            assert_eq!(nfa.is_match("ab"), None);
            assert_eq!(nfa.is_match("zabb"), Some("abb".to_owned()));
            assert_eq!(nfa.is_match("abbz"), Some("abb".to_owned()));
        }
        {
            let src = "a.+b";
            let nfa = run(src);

            assert_eq!(nfa.is_match("ab"), None);
            assert_eq!(nfa.is_match("axb"), Some("axb".to_owned()));
            assert_eq!(nfa.is_match("axbaxb"), Some("axbaxb".to_owned()));
            #[rustfmt::skip]
            assert_eq!(nfa.is_match("axaxbxb"), Some("axaxbxb".to_owned()));
            assert_eq!(nfa.is_match("baxb"), Some("axb".to_owned()));
            assert_eq!(nfa.is_match("axbz"), Some("axb".to_owned()));
        }
    }

    #[test]
    fn option() {
        {
            let src = "ab?c";
            let nfa = run(src);

            assert_eq!(nfa.is_match("ac"), Some("ac".to_owned()));
            assert_eq!(nfa.is_match("abc"), Some("abc".to_owned()));
            assert_eq!(nfa.is_match("a"), None);
            assert_eq!(nfa.is_match("zac"), Some("ac".to_owned()));
            assert_eq!(nfa.is_match("acz"), Some("ac".to_owned()));
        }
        {
            let src = "ab?";
            let nfa = run(src);

            assert_eq!(nfa.is_match("a"), Some("a".to_owned()));
            assert_eq!(nfa.is_match("ab"), Some("ab".to_owned()));
            assert_eq!(nfa.is_match("b"), None);
            assert_eq!(nfa.is_match("za"), Some("a".to_owned()));
            assert_eq!(nfa.is_match("az"), Some("a".to_owned()));
        }
    }

    #[test]
    fn positive_set() {
        {
            let src = "a[b-z]d";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abd"), Some("abd".to_owned()));
            assert_eq!(nfa.is_match("azd"), Some("azd".to_owned()));
            assert_eq!(nfa.is_match("axd"), Some("axd".to_owned()));
            assert_eq!(nfa.is_match("ad"), None);
            assert_eq!(nfa.is_match("aad"), None);
            assert_eq!(nfa.is_match("zabd"), Some("abd".to_owned()));
            assert_eq!(nfa.is_match("abdz"), Some("abd".to_owned()));
        }
        {
            let src = "[b-z]";
            let nfa = run(src);

            assert_eq!(nfa.is_match("b"), Some("b".to_owned()));
            assert_eq!(nfa.is_match("z"), Some("z".to_owned()));
            assert_eq!(nfa.is_match("x"), Some("x".to_owned()));
            assert_eq!(nfa.is_match("a"), None);
            assert_eq!(nfa.is_match("ab"), Some("b".to_owned()));
            assert_eq!(nfa.is_match("bz"), Some("b".to_owned()));
        }
        {
            let src = "[bcd]";
            let nfa = run(src);

            assert_eq!(nfa.is_match("b"), Some("b".to_owned()));
            assert_eq!(nfa.is_match("c"), Some("c".to_owned()));
            assert_eq!(nfa.is_match("d"), Some("d".to_owned()));
            assert_eq!(nfa.is_match("a"), None);
            assert_eq!(nfa.is_match("e"), None);
            assert_eq!(nfa.is_match("ab"), Some("b".to_owned()));
            assert_eq!(nfa.is_match("bz"), Some("b".to_owned()));
        }
        {
            let src = "a[bc-yz]d";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abd"), Some("abd".to_owned()));
            assert_eq!(nfa.is_match("azd"), Some("azd".to_owned()));
            assert_eq!(nfa.is_match("acd"), Some("acd".to_owned()));
            assert_eq!(nfa.is_match("ayd"), Some("ayd".to_owned()));
            assert_eq!(nfa.is_match("axd"), Some("axd".to_owned()));
            assert_eq!(nfa.is_match("aad"), None);
            assert_eq!(nfa.is_match("ad"), None);
            assert_eq!(nfa.is_match("zabd"), Some("abd".to_owned()));
            assert_eq!(nfa.is_match("abdz"), Some("abd".to_owned()));
        }
        {
            let src = "[z-z]";
            let nfa = run(src);

            assert_eq!(nfa.is_match("z"), Some("z".to_owned()));
            assert_eq!(nfa.is_match("a"), None);
            assert_eq!(nfa.is_match("az"), Some("z".to_owned()));
            assert_eq!(nfa.is_match("za"), Some("z".to_owned()));
        }
    }

    #[test]
    fn negative_set() {
        {
            let src = "a[^b-z]d";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abd"), None);
            assert_eq!(nfa.is_match("azd"), None);
            assert_eq!(nfa.is_match("axd"), None);
            assert_eq!(nfa.is_match("aad"), Some("aad".to_owned()));
            assert_eq!(nfa.is_match("ad"), None);
            assert_eq!(nfa.is_match("zaad"), Some("aad".to_owned()));
            assert_eq!(nfa.is_match("aadz"), Some("aad".to_owned()));
        }
        {
            let src = "[^b-z]";
            let nfa = run(src);

            assert_eq!(nfa.is_match("b"), None);
            assert_eq!(nfa.is_match("z"), None);
            assert_eq!(nfa.is_match("x"), None);
            assert_eq!(nfa.is_match("a"), Some("a".to_owned()));
            assert_eq!(nfa.is_match("za"), Some("a".to_owned()));
            assert_eq!(nfa.is_match("az"), Some("a".to_owned()));
        }
        {
            let src = "[^bcd]";
            let nfa = run(src);

            assert_eq!(nfa.is_match("b"), None);
            assert_eq!(nfa.is_match("c"), None);
            assert_eq!(nfa.is_match("d"), None);
            assert_eq!(nfa.is_match("a"), Some("a".to_owned()));
            assert_eq!(nfa.is_match("e"), Some("e".to_owned()));
            assert_eq!(nfa.is_match("ba"), Some("a".to_owned()));
            assert_eq!(nfa.is_match("ab"), Some("a".to_owned()));
        }
        {
            let src = "a[^bc-yz]d";
            let nfa = run(src);

            assert_eq!(nfa.is_match("abd"), None);
            assert_eq!(nfa.is_match("azd"), None);
            assert_eq!(nfa.is_match("acd"), None);
            assert_eq!(nfa.is_match("ayd"), None);
            assert_eq!(nfa.is_match("axd"), None);
            assert_eq!(nfa.is_match("aad"), Some("aad".to_owned()));
            assert_eq!(nfa.is_match("ad"), None);
            assert_eq!(nfa.is_match("zaad"), Some("aad".to_owned()));
            assert_eq!(nfa.is_match("aadz"), Some("aad".to_owned()));
        }
        {
            let src = "[^z-z]";
            let nfa = run(src);

            assert_eq!(nfa.is_match("z"), None);
            assert_eq!(nfa.is_match("a"), Some("a".to_owned()));
            assert_eq!(nfa.is_match("za"), Some("a".to_owned()));
            assert_eq!(nfa.is_match("az"), Some("a".to_owned()));
        }
    }

    #[test]
    fn pattern001() {
        {
            let src = r"[a-zA-Z0-9_\.\+\-]+@[a-zA-Z0-9_\.]+[a-zA-Z]+";
            let nfa = run(src);

            assert_eq!(
                nfa.is_match("abc@example.com"),
                Some("abc@example.com".to_owned())
            );
            assert_eq!(
                nfa.is_match("abc+123@me.example.com"),
                Some("abc+123@me.example.com".to_owned())
            );
            #[rustfmt::skip]
            assert_eq!(
                nfa.is_match("abc@example"),
                Some("abc@example".to_owned())
            );
            assert_eq!(
                nfa.is_match("abc@example.123"),
                Some("abc@example".to_owned())
            );
            assert_eq!(
                nfa.is_match("abc@def@example.com"),
                Some("abc@def".to_owned())
            );
        }
        {
            let src = r"^[a-zA-Z0-9_\.\+\-]+@[a-zA-Z0-9_\.]+[a-zA-Z]+$";
            let nfa = run(src);

            assert_eq!(
                nfa.is_match("abc@example.com"),
                Some("abc@example.com".to_owned())
            );
            assert_eq!(
                nfa.is_match("abc+123@me.example.com"),
                Some("abc+123@me.example.com".to_owned())
            );
            #[rustfmt::skip]
            assert_eq!(
                nfa.is_match("abc@example"),
                Some("abc@example".to_owned())
            );
            #[rustfmt::skip]
            assert_eq!(
                nfa.is_match("abc@example.123"),
                None,
            );
            #[rustfmt::skip]
            assert_eq!(
                nfa.is_match("abc@def@example.com"),
                None,
            );
        }
    }
}
