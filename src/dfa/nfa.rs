use crate::parser::{SyntaxKind, SyntaxNode};
use std::collections::{BTreeSet, VecDeque};

pub struct Nfa {
    pub nodes: Vec<NfaNode>,
}

#[derive(Default)]
pub struct NfaNode {
    pub nexts: Vec<NfaEdge>,
}

pub struct NfaEdge {
    pub action: NfaAction,
    pub next_id: usize,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum NfaAction {
    Asap,
    Match(char),
    MatchAny,
}

impl Nfa {
    pub fn new(node: &SyntaxNode) -> Nfa {
        let root = NfaNode::default();
        let submit = NfaNode::default();

        let mut nfa = Nfa {
            nodes: vec![root, submit],
        };
        nfa.make(node);

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

    pub fn solve_asap(&self, ids: BTreeSet<usize>) -> BTreeSet<usize> {
        let mut next_ids = BTreeSet::new();

        let mut queue_ids = VecDeque::from_iter(ids.iter());
        let mut finished = BTreeSet::new();
        while let Some(id) = queue_ids.pop_front() {
            if finished.contains(id) {
                continue;
            }
            finished.insert(*id);

            // submit node (id=1) have no edges.
            if *id == 1 {
                next_ids.insert(*id);
                continue;
            }

            for edge in &self.nodes[*id].nexts {
                match edge.action {
                    NfaAction::Asap => {
                        queue_ids.push_back(&edge.next_id);
                    }
                    NfaAction::MatchAny => {
                        next_ids.insert(*id);
                    }
                    NfaAction::Match(_) => {
                        next_ids.insert(*id);
                    }
                }
            }
        }

        next_ids
    }

    fn solve_match_char(&self, c: char, ids: BTreeSet<usize>) -> BTreeSet<usize> {
        let mut next_ids = BTreeSet::new();

        for id in ids {
            for edge in &self.nodes[id].nexts {
                match edge.action {
                    NfaAction::Asap => {
                        // nothing to do
                    }
                    NfaAction::MatchAny => {
                        next_ids.insert(edge.next_id);
                    }
                    NfaAction::Match(t) => {
                        if t == c {
                            next_ids.insert(edge.next_id);
                        }
                    }
                }
            }
        }

        next_ids
    }

    fn getid(&self) -> usize {
        self.nodes.len()
    }

    fn make(&mut self, syntax: &SyntaxNode) {
        let node_id = self.make_root(syntax, 1);
        self.nodes[0].nexts.push(NfaEdge {
            action: NfaAction::Asap,
            next_id: node_id,
        });
    }

    fn make_root(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        match syntax.kind {
            SyntaxKind::Group => self.make_group(syntax, dst_id),
            SyntaxKind::Select => self.make_select(syntax, dst_id),
            SyntaxKind::ZeroLoop => self.make_zero_loop(syntax, dst_id),
            SyntaxKind::MoreLoop => self.make_more_loop(syntax, dst_id),
            SyntaxKind::Option => self.make_option(syntax, dst_id),
            SyntaxKind::MatchAny => self.make_match_any(dst_id),
            SyntaxKind::Match(c) => self.make_match_char(c, dst_id),
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

    fn make_select(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let node_id = self.getid();
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

    fn make_zero_loop(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let loop_id = self.getid();
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

    fn make_more_loop(&mut self, syntax: &SyntaxNode, dst_id: usize) -> usize {
        let loop_id = self.getid();
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
        let node_id = self.getid();
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
        let node_id = self.getid();
        self.nodes.push(NfaNode {
            nexts: vec![NfaEdge {
                action: NfaAction::MatchAny,
                next_id: dst_id,
            }],
        });

        node_id
    }

    fn make_match_char(&mut self, c: char, dst_id: usize) -> usize {
        let node_id = self.getid();
        self.nodes.push(NfaNode {
            nexts: vec![NfaEdge {
                action: NfaAction::Match(c),
                next_id: dst_id,
            }],
        });

        node_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn basic() {
        let src = "abc".to_owned();

        let mut parser = Parser::new(src);
        let result = parser.parse();
        let matcher = Nfa::new(&result.unwrap());

        assert_eq!(matcher.is_match("abc".to_owned()), true);
        assert_eq!(matcher.is_match("abcd".to_owned()), false);
        assert_eq!(matcher.is_match("ab".to_owned()), false);
    }

    #[test]
    fn select() {
        let src = "abc|def|ghi".to_owned();

        let mut parser = Parser::new(src);
        let result = parser.parse();
        let matcher = Nfa::new(&result.unwrap());

        assert_eq!(matcher.is_match("abc".to_owned()), true);
        assert_eq!(matcher.is_match("def".to_owned()), true);
        assert_eq!(matcher.is_match("ghi".to_owned()), true);
        assert_eq!(matcher.is_match("abcdefghi".to_owned()), false);
    }

    #[test]
    fn zero_loop() {
        {
            let src = "ab*c".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Nfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("abbbc".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), true);
            assert_eq!(matcher.is_match("ab".to_owned()), false);
        }
        {
            let src = "ab*".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Nfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("ab".to_owned()), true);
            assert_eq!(matcher.is_match("abbb".to_owned()), true);
            assert_eq!(matcher.is_match("a".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), false);
        }
    }

    #[test]
    fn more_loop() {
        {
            let src = "ab+c".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Nfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("abbbc".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), false);
        }
        {
            let src = "ab+".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Nfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("ab".to_owned()), true);
            assert_eq!(matcher.is_match("abbb".to_owned()), true);
            assert_eq!(matcher.is_match("a".to_owned()), false);
            assert_eq!(matcher.is_match("ac".to_owned()), false);
        }
    }

    #[test]
    fn option() {
        {
            let src = "ab?c".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Nfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), true);
            assert_eq!(matcher.is_match("abbc".to_owned()), false);
        }
        {
            let src = "ab?".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Nfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("ab".to_owned()), true);
            assert_eq!(matcher.is_match("a".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), false);
        }
    }

    #[test]
    fn match_any() {
        {
            let src = "a.c".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Nfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("adc".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), false);
            assert_eq!(matcher.is_match("abbc".to_owned()), false);
        }
        {
            let src = "a.".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Nfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("ab".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), true);
            assert_eq!(matcher.is_match("a".to_owned()), false);
        }
    }

    #[test]
    fn match_group() {
        {
            let src = "a(bc)d".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Nfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("abcd".to_owned()), true);
            assert_eq!(matcher.is_match("abd".to_owned()), false);
            assert_eq!(matcher.is_match("ad".to_owned()), false);
        }
        {
            let src = "a(bc)".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Nfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("ab".to_owned()), false);
            assert_eq!(matcher.is_match("a".to_owned()), false);
        }
    }

    #[test]
    fn pattern001() {
        let src = "(https?|ftp):(exp.)?+".to_owned();

        let mut parser = Parser::new(src);
        let result = parser.parse();
        let matcher = Nfa::new(&result.unwrap());

        assert_eq!(matcher.is_match("http:exp_".to_owned()), true);
        assert_eq!(matcher.is_match("https:exp_".to_owned()), true);
        assert_eq!(matcher.is_match("ftp:exp_".to_owned()), true);
        assert_eq!(matcher.is_match("ftp:".to_owned()), true);
        assert_eq!(matcher.is_match("ftp:exp.exp_".to_owned()), true);
        assert_eq!(matcher.is_match("ftp:exp.exp".to_owned()), false);
    }
}