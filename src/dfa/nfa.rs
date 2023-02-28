use crate::parser::{SyntaxKind, SyntaxNode};
use std::collections::{BTreeSet, VecDeque};

pub struct Nfa {
    pub nodes: Vec<NfaNode>,
}

pub struct NfaNode {
    pub nexts: Vec<NfaEdge>,
}

pub struct NfaEdge {
    pub action: NfaAction,
    pub next_id: usize,
}

pub enum NfaAction {
    Asap,
    Match(char),
    MatchAny,
    MatchSet(Vec<NfaMatchAction>),
    UnmatchSet(Vec<NfaMatchAction>),
}

#[derive(PartialEq, PartialOrd, Eq, Ord)]
pub enum NfaMatchAction {
    Char(char),
    Range(char, char),
}

impl Nfa {
    pub fn new(syntax: &SyntaxNode) -> Nfa {
        let mut nfa = Nfa {
            nodes: vec![
                NfaNode { nexts: vec![] }, // root
                NfaNode { nexts: vec![] }, // submit
            ],
        };
        nfa.make(syntax);

        nfa
    }

    pub fn is_match(&self, str: String) -> bool {
        let mut current = BTreeSet::new();
        current.insert(0);
        current = self.solve_asap(current);

        for c in str.chars() {
            if current.is_empty() {
                return false;
            }
            current = self.solve_match_char(c, current);
            current = self.solve_asap(current);
        }

        current.contains(&1)
    }

    fn solve_match_char(&self, c: char, current_ids: BTreeSet<usize>) -> BTreeSet<usize> {
        let mut next_ids = BTreeSet::new();
        for id in current_ids {
            for edge in &self.nodes[id].nexts {
                match &edge.action {
                    NfaAction::Match(t) => {
                        if *t == c {
                            next_ids.insert(edge.next_id);
                        }
                    }
                    NfaAction::MatchAny => {
                        next_ids.insert(edge.next_id);
                    }
                    NfaAction::MatchSet(set) => {
                        if set.iter().any(|m| match m {
                            NfaMatchAction::Char(t) => *t == c,
                            NfaMatchAction::Range(a, b) => *a <= c && c <= *b,
                        }) {
                            next_ids.insert(edge.next_id);
                        }
                    }
                    NfaAction::UnmatchSet(set) => {
                        if set.iter().all(|m| match m {
                            NfaMatchAction::Char(t) => *t != c,
                            NfaMatchAction::Range(a, b) => c < *a || *b < c,
                        }) {
                            next_ids.insert(edge.next_id);
                        }
                    }
                    _ => {
                        // nothing todo
                    }
                }
            }
        }
        next_ids
    }

    fn solve_asap(&self, current_ids: BTreeSet<usize>) -> BTreeSet<usize> {
        let mut next_ids = BTreeSet::new();
        let mut queue_ids = VecDeque::from_iter(current_ids.iter());
        let mut finished = BTreeSet::new();
        while let Some(id) = queue_ids.pop_front() {
            if finished.contains(id) {
                continue;
            }
            finished.insert(*id);

            // submit node (id=1).
            if *id == 1 {
                next_ids.insert(*id);
                continue;
            }

            for edge in &self.nodes[*id].nexts {
                match edge.action {
                    NfaAction::Asap => {
                        queue_ids.push_back(&edge.next_id);
                    }
                    _ => {
                        next_ids.insert(*id);
                    }
                }
            }
        }
        next_ids
    }

    /// `make` recursively parses the `syntax` and constructs the NFA.
    fn make(&mut self, syntax: &SyntaxNode) {
        // build the NFA recursively from root and receive the next start-node-ID.
        let node_id = self.make_root(syntax, 1);

        // bind the next node to the first node.
        self.nodes[0].nexts.push(NfaEdge {
            action: NfaAction::Asap,
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
        self.nodes.push(NfaNode { nexts: vec![] });

        for child in syntax.children.iter() {
            let match_id = self.make_root(child, dst_id);
            self.nodes[node_id].nexts.push(NfaEdge {
                action: NfaAction::Asap,
                next_id: match_id,
            });
        }
        node_id
    }

    fn make_many_star(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let loop_id = self.nodes.len();
        self.nodes.push(NfaNode {
            nexts: vec![NfaEdge {
                action: NfaAction::Asap,
                next_id: dst_id,
            }],
        });

        let match_id = self.make_root(&syntax.children[0], loop_id);
        self.nodes[loop_id].nexts.push(NfaEdge {
            action: NfaAction::Asap,
            next_id: match_id,
        });
        loop_id
    }

    fn make_many_plus(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let loop_id = self.nodes.len();
        self.nodes.push(NfaNode {
            nexts: vec![NfaEdge {
                action: NfaAction::Asap,
                next_id: dst_id,
            }],
        });

        let match_id = self.make_root(&syntax.children[0], loop_id);
        self.nodes[loop_id].nexts.push(NfaEdge {
            action: NfaAction::Asap,
            next_id: match_id,
        });
        match_id
    }

    fn make_option(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let node_id = self.nodes.len();
        self.nodes.push(NfaNode {
            nexts: vec![NfaEdge {
                action: NfaAction::Asap,
                next_id: dst_id,
            }],
        });

        let match_id = self.make_root(&syntax.children[0], dst_id);
        self.nodes[node_id].nexts.push(NfaEdge {
            action: NfaAction::Asap,
            next_id: match_id,
        });
        node_id
    }

    fn make_match_any(&mut self, dst_id: usize) -> usize {
        let node_id = self.nodes.len();
        self.nodes.push(NfaNode {
            nexts: vec![NfaEdge {
                action: NfaAction::MatchAny,
                next_id: dst_id,
            }],
        });
        node_id
    }

    fn make_match_char(&mut self, c: char, dst_id: usize) -> usize {
        let node_id = self.nodes.len();
        self.nodes.push(NfaNode {
            nexts: vec![NfaEdge {
                action: NfaAction::Match(c),
                next_id: dst_id,
            }],
        });
        node_id
    }

    fn make_positive_set(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let set = self.make_set_items(syntax).into_iter().collect::<Vec<_>>();

        let node_id = self.nodes.len();
        self.nodes.push(NfaNode {
            nexts: vec![NfaEdge {
                action: NfaAction::MatchSet(set),
                next_id: dst_id,
            }],
        });
        node_id
    }

    fn make_negative_set(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let set = self.make_set_items(syntax).into_iter().collect::<Vec<_>>();

        let node_id = self.nodes.len();
        self.nodes.push(NfaNode {
            nexts: vec![NfaEdge {
                action: NfaAction::UnmatchSet(set),
                next_id: dst_id,
            }],
        });
        node_id
    }

    fn make_set_items(&mut self, syntax: &SyntaxNode) -> BTreeSet<NfaMatchAction> {
        let mut set = BTreeSet::new();
        for child in syntax.children.iter() {
            match child.kind {
                SyntaxKind::Group => {
                    let res = self.make_set_items(child);
                    set.extend(res.into_iter());
                }
                SyntaxKind::Match(c) => {
                    set.insert(NfaMatchAction::Char(c));
                }
                SyntaxKind::MatchRange(a, b) => {
                    set.insert(NfaMatchAction::Range(a, b));
                }
                _ => unreachable!(),
            };
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn run(pattern: &str) -> Nfa {
        let mut parser = Parser::new(pattern.to_owned());
        let result = parser.parse();
        Nfa::new(&result.unwrap())
    }

    #[test]
    fn match_char() {
        let src = "abc";
        let matcher = run(src);

        assert_eq!(matcher.is_match("abc".to_owned()), true);
        assert_eq!(matcher.is_match("abcd".to_owned()), false);
        assert_eq!(matcher.is_match("ab".to_owned()), false);
    }

    #[test]
    fn match_metachar() {
        let src = r"a\+c";
        let matcher = run(src);

        assert_eq!(matcher.is_match("a+c".to_owned()), true);
        assert_eq!(matcher.is_match("aac".to_owned()), false);
        assert_eq!(matcher.is_match("ac".to_owned()), false);
    }

    #[test]
    fn match_any() {
        {
            let src = "a.c";
            let matcher = run(src);

            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("adc".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), false);
            assert_eq!(matcher.is_match("abbc".to_owned()), false);
        }
        {
            let src = "a.";
            let matcher = run(src);

            assert_eq!(matcher.is_match("ab".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), true);
            assert_eq!(matcher.is_match("a".to_owned()), false);
        }
    }

    #[test]
    fn group() {
        {
            let src = "a(bc)d";
            let matcher = run(src);

            assert_eq!(matcher.is_match("abcd".to_owned()), true);
            assert_eq!(matcher.is_match("abc".to_owned()), false);
            assert_eq!(matcher.is_match("ad".to_owned()), false);
        }
        {
            let src = "a(bc)";
            let matcher = run(src);

            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("ab".to_owned()), false);
            assert_eq!(matcher.is_match("a".to_owned()), false);
        }
    }

    #[test]
    fn union() {
        let src = "abc|def|ghi";
        let matcher = run(src);

        assert_eq!(matcher.is_match("abc".to_owned()), true);
        assert_eq!(matcher.is_match("def".to_owned()), true);
        assert_eq!(matcher.is_match("ghi".to_owned()), true);
        assert_eq!(matcher.is_match("ab".to_owned()), false);
        assert_eq!(matcher.is_match("hi".to_owned()), false);
    }

    #[test]
    fn many_star() {
        {
            let src = "ab*c";
            let matcher = run(src);

            assert_eq!(matcher.is_match("ac".to_owned()), true);
            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("abbc".to_owned()), true);
            assert_eq!(matcher.is_match("abbbc".to_owned()), true);
            assert_eq!(matcher.is_match("ab".to_owned()), false);
        }
        {
            let src = "ab*";
            let matcher = run(src);

            assert_eq!(matcher.is_match("a".to_owned()), true);
            assert_eq!(matcher.is_match("ab".to_owned()), true);
            assert_eq!(matcher.is_match("abb".to_owned()), true);
            assert_eq!(matcher.is_match("abbb".to_owned()), true);
            assert_eq!(matcher.is_match("b".to_owned()), false);
        }
    }

    #[test]
    fn many_plus() {
        {
            let src = "ab+c";
            let matcher = run(src);

            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("abbc".to_owned()), true);
            assert_eq!(matcher.is_match("abbbc".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), false);
            assert_eq!(matcher.is_match("ab".to_owned()), false);
        }
        {
            let src = "ab+";
            let matcher = run(src);

            assert_eq!(matcher.is_match("ab".to_owned()), true);
            assert_eq!(matcher.is_match("abb".to_owned()), true);
            assert_eq!(matcher.is_match("abbb".to_owned()), true);
            assert_eq!(matcher.is_match("a".to_owned()), false);
            assert_eq!(matcher.is_match("b".to_owned()), false);
        }
    }

    #[test]
    fn option() {
        {
            let src = "ab?c";
            let matcher = run(src);

            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), true);
            assert_eq!(matcher.is_match("a".to_owned()), false);
        }
        {
            let src = "ab?";
            let matcher = run(src);

            assert_eq!(matcher.is_match("ab".to_owned()), true);
            assert_eq!(matcher.is_match("a".to_owned()), true);
            assert_eq!(matcher.is_match("b".to_owned()), false);
        }
    }

    #[test]
    fn positive_set() {
        {
            let src = "a[b-z]d";
            let matcher = run(src);

            assert_eq!(matcher.is_match("abd".to_owned()), true);
            assert_eq!(matcher.is_match("azd".to_owned()), true);
            assert_eq!(matcher.is_match("axd".to_owned()), true);
            assert_eq!(matcher.is_match("ad".to_owned()), false);
            assert_eq!(matcher.is_match("aad".to_owned()), false);
        }
        {
            let src = "[b-z]";
            let matcher = run(src);

            assert_eq!(matcher.is_match("b".to_owned()), true);
            assert_eq!(matcher.is_match("z".to_owned()), true);
            assert_eq!(matcher.is_match("x".to_owned()), true);
            assert_eq!(matcher.is_match("a".to_owned()), false);
        }
        {
            let src = "[bcd]";
            let matcher = run(src);

            assert_eq!(matcher.is_match("b".to_owned()), true);
            assert_eq!(matcher.is_match("c".to_owned()), true);
            assert_eq!(matcher.is_match("d".to_owned()), true);
            assert_eq!(matcher.is_match("a".to_owned()), false);
            assert_eq!(matcher.is_match("e".to_owned()), false);
        }
        {
            let src = "a[bc-yz]d";
            let matcher = run(src);

            assert_eq!(matcher.is_match("abd".to_owned()), true);
            assert_eq!(matcher.is_match("azd".to_owned()), true);
            assert_eq!(matcher.is_match("acd".to_owned()), true);
            assert_eq!(matcher.is_match("ayd".to_owned()), true);
            assert_eq!(matcher.is_match("axd".to_owned()), true);
            assert_eq!(matcher.is_match("ad".to_owned()), false);
            assert_eq!(matcher.is_match("aad".to_owned()), false);
        }
        {
            let src = "[z-z]";
            let matcher = run(src);

            assert_eq!(matcher.is_match("z".to_owned()), true);
            assert_eq!(matcher.is_match("a".to_owned()), false);
        }
    }

    #[test]
    fn negative_set() {
        {
            let src = "a[^b-z]d";
            let matcher = run(src);

            assert_eq!(matcher.is_match("abd".to_owned()), false);
            assert_eq!(matcher.is_match("azd".to_owned()), false);
            assert_eq!(matcher.is_match("axd".to_owned()), false);
            assert_eq!(matcher.is_match("ad".to_owned()), false);
            assert_eq!(matcher.is_match("aad".to_owned()), true);
        }
        {
            let src = "[^b-z]";
            let matcher = run(src);

            assert_eq!(matcher.is_match("b".to_owned()), false);
            assert_eq!(matcher.is_match("z".to_owned()), false);
            assert_eq!(matcher.is_match("x".to_owned()), false);
            assert_eq!(matcher.is_match("a".to_owned()), true);
        }
        {
            let src = "[^bcd]";
            let matcher = run(src);

            assert_eq!(matcher.is_match("b".to_owned()), false);
            assert_eq!(matcher.is_match("c".to_owned()), false);
            assert_eq!(matcher.is_match("d".to_owned()), false);
            assert_eq!(matcher.is_match("a".to_owned()), true);
            assert_eq!(matcher.is_match("e".to_owned()), true);
        }
        {
            let src = "a[^bc-yz]d";
            let matcher = run(src);

            assert_eq!(matcher.is_match("abd".to_owned()), false);
            assert_eq!(matcher.is_match("azd".to_owned()), false);
            assert_eq!(matcher.is_match("acd".to_owned()), false);
            assert_eq!(matcher.is_match("ayd".to_owned()), false);
            assert_eq!(matcher.is_match("axd".to_owned()), false);
            assert_eq!(matcher.is_match("ad".to_owned()), false);
            assert_eq!(matcher.is_match("aad".to_owned()), true);
        }
        {
            let src = "[^z-z]";
            let matcher = run(src);

            assert_eq!(matcher.is_match("z".to_owned()), false);
            assert_eq!(matcher.is_match("a".to_owned()), true);
        }
    }

    #[test]
    fn pattern001() {
        let src = r"[a-zA-Z0-9_\.\+\-]+@[a-zA-Z0-9_\.]+[a-zA-Z]+";
        let matcher = run(src);

        assert_eq!(matcher.is_match("abc@example.com".to_owned()), true);
        assert_eq!(matcher.is_match("abc+123@me.example.com".to_owned()), true);
        assert_eq!(matcher.is_match("abc@example".to_owned()), true);
        assert_eq!(matcher.is_match("abc@example.123".to_owned()), false);
        assert_eq!(matcher.is_match("abc@def@example.com".to_owned()), false);
    }
}
