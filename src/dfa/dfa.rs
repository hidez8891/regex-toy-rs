use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::parser::SyntaxNode;

use super::nfa::{Nfa, NfaAction};

pub struct Dfa {
    ids: BTreeMap<BTreeSet<usize>, usize>,
    nodes: Vec<DfaNode>,
}

struct DfaNode {
    trans: BTreeMap<char, usize>,
    any_trans: Option<usize>,
    is_submit: bool,
}

impl Dfa {
    pub fn new(node: &SyntaxNode) -> Dfa {
        let nfa = Nfa::new(node);

        let mut dfa = Dfa {
            ids: BTreeMap::new(),
            nodes: vec![],
        };
        dfa.make(&nfa);

        dfa
    }

    pub fn is_match(&self, str: String) -> bool {
        let mut node = &self.nodes[0];
        for c in str.chars() {
            match node.trans.get(&c).or(node.any_trans.as_ref()) {
                Some(id) => {
                    node = &self.nodes[*id];
                }
                _ => {
                    return false;
                }
            }
        }

        node.is_submit
    }

    fn make(&mut self, nfa: &Nfa) {
        // initial state
        let mut root_nfa_ids = BTreeSet::new();
        root_nfa_ids.insert(0 as usize);
        root_nfa_ids = nfa.solve_asap(root_nfa_ids);

        self.ids.insert(root_nfa_ids.clone(), 0);
        self.nodes.push(DfaNode {
            trans: BTreeMap::new(),
            any_trans: None,
            is_submit: false,
        });

        // generate transition map
        let mut queue_nfa_ids = VecDeque::new();
        queue_nfa_ids.push_back(root_nfa_ids);

        let mut finished = BTreeSet::new();
        while let Some(nfa_ids) = queue_nfa_ids.pop_front() {
            if finished.contains(&nfa_ids) {
                continue;
            }
            finished.insert(nfa_ids.clone());

            // calc translation map [Action] -> [NFA Node ID]
            let mut trans_map: BTreeMap<&NfaAction, BTreeSet<usize>> = BTreeMap::new();
            for nfa_id in &nfa_ids {
                for edge in &nfa.nodes[*nfa_id].nexts {
                    match edge.action {
                        NfaAction::Asap => {
                            // nothing to do
                        }
                        NfaAction::MatchAny | NfaAction::Match(_) => {
                            trans_map
                                .entry(&edge.action)
                                .or_insert(BTreeSet::new())
                                .insert(edge.next_id);
                        }
                    }
                }
            }

            // update DFA translation map
            let dfa_id = self.ids[&nfa_ids];
            for (action, next_nfa_ids) in trans_map.iter_mut() {
                *next_nfa_ids = nfa.solve_asap(std::mem::take(next_nfa_ids));
                queue_nfa_ids.push_back(next_nfa_ids.clone());

                // generate new DFA Node
                if !self.ids.contains_key(next_nfa_ids) {
                    let new_dfa_id = self.nodes.len();
                    self.ids.insert(next_nfa_ids.clone(), new_dfa_id);
                    self.nodes.push(DfaNode {
                        trans: BTreeMap::new(),
                        any_trans: None,
                        is_submit: next_nfa_ids.contains(&1),
                    });
                }

                // update translation map [CHAR] -> [DFA Node ID] or any translation map
                let mut dfa_node = self.nodes.get_mut(dfa_id).unwrap();
                match action {
                    NfaAction::MatchAny => {
                        dfa_node.any_trans = Some(self.ids[next_nfa_ids]);
                    }
                    NfaAction::Match(c) => {
                        dfa_node.trans.insert(*c, self.ids[next_nfa_ids]);
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }
        }
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
        let matcher = Dfa::new(&result.unwrap());

        assert_eq!(matcher.is_match("abc".to_owned()), true);
        assert_eq!(matcher.is_match("abcd".to_owned()), false);
        assert_eq!(matcher.is_match("ab".to_owned()), false);
    }

    #[test]
    fn select() {
        let src = "abc|def|ghi".to_owned();

        let mut parser = Parser::new(src);
        let result = parser.parse();
        let matcher = Dfa::new(&result.unwrap());

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
            let matcher = Dfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("abbbc".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), true);
            assert_eq!(matcher.is_match("ab".to_owned()), false);
        }
        {
            let src = "ab*".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Dfa::new(&result.unwrap());

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
            let matcher = Dfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("abbbc".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), false);
        }
        {
            let src = "ab+".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Dfa::new(&result.unwrap());

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
            let matcher = Dfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), true);
            assert_eq!(matcher.is_match("abbc".to_owned()), false);
        }
        {
            let src = "ab?".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Dfa::new(&result.unwrap());

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
            let matcher = Dfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("abc".to_owned()), true);
            assert_eq!(matcher.is_match("adc".to_owned()), true);
            assert_eq!(matcher.is_match("ac".to_owned()), false);
            assert_eq!(matcher.is_match("abbc".to_owned()), false);
        }
        {
            let src = "a.".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Dfa::new(&result.unwrap());

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
            let matcher = Dfa::new(&result.unwrap());

            assert_eq!(matcher.is_match("abcd".to_owned()), true);
            assert_eq!(matcher.is_match("abd".to_owned()), false);
            assert_eq!(matcher.is_match("ad".to_owned()), false);
        }
        {
            let src = "a(bc)".to_owned();

            let mut parser = Parser::new(src);
            let result = parser.parse();
            let matcher = Dfa::new(&result.unwrap());

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
        let matcher = Dfa::new(&result.unwrap());

        assert_eq!(matcher.is_match("http:exp_".to_owned()), true);
        assert_eq!(matcher.is_match("https:exp_".to_owned()), true);
        assert_eq!(matcher.is_match("ftp:exp_".to_owned()), true);
        assert_eq!(matcher.is_match("ftp:".to_owned()), true);
        assert_eq!(matcher.is_match("ftp:exp.exp_".to_owned()), true);
        assert_eq!(matcher.is_match("ftp:exp.exp".to_owned()), false);
    }
}
