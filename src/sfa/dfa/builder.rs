use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use super::dfa::{Dfa, IndexSet, Node, Transition};
use crate::sfa::nfa::nfa;

pub(crate) struct Builder<'a> {
    nfa_nodes: &'a Vec<nfa::Node>,
    dfa_nodes: Vec<Node>,
    dfa_nodemap: HashMap<IndexSet, usize>,
}

impl<'a> Builder<'a> {
    pub fn build(nfa: &'a nfa::Nfa) -> Dfa {
        let mut builder = Builder {
            nfa_nodes: &nfa.nodes,
            dfa_nodes: Vec::new(),
            dfa_nodemap: HashMap::new(),
        };

        builder.build_();

        return Dfa {
            nodes: builder.dfa_nodes,
            nodemap: builder.dfa_nodemap,
        };
    }

    fn build_(&mut self) {
        let mut q = VecDeque::new();
        {
            let mut index = IndexSet::new();
            index.insert(0);
            index = self.resolve_empty_trans(&index);
            q.push_back(index);
        }

        while let Some(index) = q.pop_front() {
            if index.is_empty() {
                continue;
            }
            if self.dfa_nodemap.contains_key(&index) {
                continue;
            }

            let is_match = index.contains(&1);

            let mut trans = Transition::new(256);
            for i in index.iter() {
                let trans_map = self.build_trans_map(&self.nfa_nodes[*i], is_match);
                trans.merge(&trans_map);
            }

            let uniq_index_list: HashSet<_> = trans.trans.iter().cloned().collect();
            q.extend(uniq_index_list.into_iter());
            if !trans.start_line.is_empty() {
                q.push_back(trans.start_line.clone());
            }
            if !trans.end_line.is_empty() {
                q.push_back(trans.end_line.clone());
            }

            self.dfa_nodemap.insert(index, self.dfa_nodes.len());
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
                nfa::EdgeAction::Asap => { /* nothing */ }
                nfa::EdgeAction::Match(c) => {
                    trans.trans[*c as usize].insert(edge.next_id);
                }
                nfa::EdgeAction::MatchAny => {
                    for tmap in trans.trans.iter_mut() {
                        tmap.insert(edge.next_id);
                    }
                }
                nfa::EdgeAction::MatchIncludeSet(set) => {
                    for m in set.iter() {
                        match m {
                            nfa::MatchSet::Char(c) => {
                                trans.trans[*c as usize].insert(edge.next_id);
                            }
                            nfa::MatchSet::Range(a, b) => {
                                for c in *a..=*b {
                                    trans.trans[c as usize].insert(edge.next_id);
                                }
                            }
                        }
                    }
                }
                nfa::EdgeAction::MatchExcludeSet(set) => {
                    let mut exclude_trans = Transition::new(256);
                    let mut next_nodes = IndexSet::new();

                    // calc exclude transition-map
                    for m in set.iter() {
                        match m {
                            nfa::MatchSet::Char(c) => {
                                exclude_trans.trans[*c as usize].insert(edge.next_id);
                                next_nodes.insert(edge.next_id);
                            }
                            nfa::MatchSet::Range(a, b) => {
                                for c in *a..=*b {
                                    exclude_trans.trans[c as usize].insert(edge.next_id);
                                    next_nodes.insert(edge.next_id);
                                }
                            }
                        }
                    }

                    // apply exclude transition-map
                    for (i, set1) in trans.trans.iter_mut().enumerate() {
                        let set2 = &exclude_trans.trans[i];
                        *set1 = &*set1 | &(&next_nodes - set2);
                    }
                }
                nfa::EdgeAction::MatchSOL => {
                    trans.start_line.insert(edge.next_id);
                }
                nfa::EdgeAction::MatchEOL => {
                    trans.end_line.insert(edge.next_id);
                }
            }
        }

        // resolve empty transition
        for index in trans.trans.iter_mut() {
            *index = self.resolve_empty_trans(index);
        }
        trans.start_line = self.resolve_empty_trans(&trans.start_line);
        trans.end_line = self.resolve_empty_trans(&trans.end_line);

        trans
    }

    fn resolve_empty_trans(&self, index: &IndexSet) -> IndexSet {
        let mut result_index = BTreeSet::new();

        let mut q: VecDeque<_> = index.iter().collect();
        while let Some(i) = q.pop_front() {
            if !result_index.insert(*i) {
                continue;
            }

            for edge in self.nfa_nodes[*i].nexts.iter() {
                match edge.action {
                    nfa::EdgeAction::Asap => {
                        q.push_back(&edge.next_id);
                    }
                    _ => { /* nothing */ }
                }
            }
        }

        result_index
    }
}
