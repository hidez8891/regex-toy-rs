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
        match syntax.kind {
            SyntaxKind::Group => self.make_group(syntax, dst_id),
            SyntaxKind::Union => self.make_union(syntax, dst_id),
            SyntaxKind::LongestStar => self.make_long_star(syntax, dst_id),
            SyntaxKind::LongestPlus => self.make_long_plus(syntax, dst_id),
            SyntaxKind::ShortestStar => self.make_short_star(syntax, dst_id),
            SyntaxKind::ShortestPlus => self.make_short_plus(syntax, dst_id),
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

    fn make_long_star(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
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

    fn make_long_plus(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
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

    fn make_short_star(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let loop_id = self.nodes.len();
        self.nodes.push(Node {
            nexts: vec![Edge {
                action: EdgeAction::Asap,
                next_id: dst_id,
            }],
        });

        let match_id = self.make_root(&syntax.children[0], loop_id);
        self.nodes[loop_id].nexts.push(Edge {
            action: EdgeAction::Asap,
            next_id: match_id,
        });
        loop_id
    }

    fn make_short_plus(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let loop_id = self.nodes.len();
        self.nodes.push(Node {
            nexts: vec![Edge {
                action: EdgeAction::Asap,
                next_id: dst_id,
            }],
        });

        let match_id = self.make_root(&syntax.children[0], loop_id);
        self.nodes[loop_id].nexts.push(Edge {
            action: EdgeAction::Asap,
            next_id: match_id,
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

    #[allow(clippy::only_used_in_recursion)]
    fn make_set_items(&self, syntax: &SyntaxNode) -> BTreeSet<MatchSetItem> {
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

    #[test]
    fn long_star() {
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
    fn long_plus() {
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
    fn short_star() {
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
    fn short_plus() {
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
    fn positive_set() {
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

    #[test]
    fn negative_set() {
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
