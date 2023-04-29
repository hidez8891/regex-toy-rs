use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use super::{Dfa, IndexSet, Node, Transition};
use crate::sfa::nfa;

pub(crate) struct Builder {
    nfa: nfa::Nfa,
    dfa_nodes: Vec<Node>,
    dfa_indexmap: HashMap<IndexSet, usize>,
}

impl Builder {
    pub fn build(nfa: nfa::Nfa) -> Dfa {
        let mut builder = Builder {
            nfa,
            dfa_nodes: Vec::new(),
            dfa_indexmap: HashMap::new(),
        };

        builder.build_();

        return Dfa {
            nfa: builder.nfa,
            nodes: builder.dfa_nodes,
            indexmap: builder.dfa_indexmap,
        };
    }

    fn build_(&mut self) {
        let mut q = VecDeque::new();
        {
            let mut index = IndexSet::new();
            index.insert(0);
            index = self.resolve_empty_transition(&index);
            q.push_back(index);
        }

        while let Some(index) = q.pop_front() {
            if index.is_empty() {
                continue;
            }
            if self.dfa_indexmap.contains_key(&index) {
                continue;
            }

            let is_match = index.contains(&1);

            let mut trans = Transition::new(256);
            for i in index.iter() {
                let trans_map = self.build_trans_map(&self.nfa.nodes[*i], is_match);
                trans.merge(&trans_map);
            }

            let uniq_index_list: HashSet<_> = trans.table.iter().cloned().collect();
            q.extend(uniq_index_list.into_iter());
            if !trans.sol_next_index.is_empty() {
                q.push_back(trans.sol_next_index.clone());
            }
            if !trans.eol_next_index.is_empty() {
                q.push_back(trans.eol_next_index.clone());
            }

            self.dfa_indexmap.insert(index, self.dfa_nodes.len());
            self.dfa_nodes.push(Node { trans, is_match });
        }
    }

    fn build_trans_map(&self, node: &nfa::Node, is_match: bool) -> Transition {
        let mut trans = Transition::new(256);

        for edge in node.nexts.iter() {
            if is_match && !edge.is_greedy {
                continue;
            }

            match &edge.action {
                nfa::EdgeAction::Asap
                | nfa::EdgeAction::CaptureStart(_)
                | nfa::EdgeAction::CaptureEnd(_) => { /* nothing */ }
                nfa::EdgeAction::Match(c) => {
                    trans.table[*c as usize].insert(edge.next_id);
                }
                nfa::EdgeAction::MatchAny => {
                    for indexset in trans.table.iter_mut() {
                        indexset.insert(edge.next_id);
                    }
                }
                nfa::EdgeAction::MatchIncludeSet(set) => {
                    for m in set.iter() {
                        match m {
                            nfa::MatchSet::Char(c) => {
                                trans.table[*c as usize].insert(edge.next_id);
                            }
                            nfa::MatchSet::Range(a, b) => {
                                for c in *a..=*b {
                                    trans.table[c as usize].insert(edge.next_id);
                                }
                            }
                        }
                    }
                }
                nfa::EdgeAction::MatchExcludeSet(set) => {
                    let mut exclude_table = Transition::new(256);
                    let mut next_indexset = IndexSet::new();

                    // calc exclude transition-map
                    for m in set.iter() {
                        match m {
                            nfa::MatchSet::Char(c) => {
                                exclude_table.table[*c as usize].insert(edge.next_id);
                                next_indexset.insert(edge.next_id);
                            }
                            nfa::MatchSet::Range(a, b) => {
                                for c in *a..=*b {
                                    exclude_table.table[c as usize].insert(edge.next_id);
                                    next_indexset.insert(edge.next_id);
                                }
                            }
                        }
                    }

                    // apply exclude transition-map
                    for (i, indexset) in trans.table.iter_mut().enumerate() {
                        let exclude_set = &exclude_table.table[i];
                        *indexset = &*indexset | &(&next_indexset - exclude_set);
                    }
                }
                nfa::EdgeAction::MatchSOL => {
                    trans.sol_next_index.insert(edge.next_id);
                }
                nfa::EdgeAction::MatchEOL => {
                    trans.eol_next_index.insert(edge.next_id);
                }
            }
        }

        // resolve empty transition
        for index in trans.table.iter_mut() {
            *index = self.resolve_empty_transition(index);
        }
        trans.sol_next_index = self.resolve_empty_transition(&trans.sol_next_index);
        trans.eol_next_index = self.resolve_empty_transition(&trans.eol_next_index);

        trans
    }

    fn resolve_empty_transition(&self, index: &IndexSet) -> IndexSet {
        let mut result_index = BTreeSet::new();

        let mut q: VecDeque<_> = index.iter().collect();
        while let Some(i) = q.pop_front() {
            if !result_index.insert(*i) {
                continue;
            }

            for edge in self.nfa.nodes[*i].nexts.iter() {
                match edge.action {
                    nfa::EdgeAction::Asap
                    | nfa::EdgeAction::CaptureStart(_)
                    | nfa::EdgeAction::CaptureEnd(_) => {
                        q.push_back(&edge.next_id);
                    }
                    _ => { /* nothing */ }
                }
            }
        }

        result_index
    }
}
