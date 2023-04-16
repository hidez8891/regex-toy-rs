use std::collections::{BTreeSet, HashMap};

use super::{builder::Builder, matcher::Matcher};
use crate::sfa::Nfa;

pub struct Dfa {
    pub(crate) nodes: Vec<Node>,
    pub(crate) indexmap: HashMap<IndexSet, usize>,
}

impl Dfa {
    pub fn new(pattern: &str) -> Result<Dfa, String> {
        let nfa = Nfa::new(pattern)?;
        let dfa = Builder::build(&nfa);

        Ok(dfa)
    }

    pub fn is_match<'a>(&self, str: &'a str) -> Option<&'a str> {
        let mut matcher = Matcher::new(&self);
        matcher.execute(str)
    }
}

pub(crate) struct Node {
    pub trans: Transition,
    pub is_match: bool,
}

pub(crate) struct Transition {
    pub table: Vec<IndexSet>,
    pub sol_next_index: IndexSet,
    pub eol_next_index: IndexSet,
}

impl Transition {
    pub fn new(size: usize) -> Self {
        let mut table = Vec::with_capacity(size);
        for _ in 0..size {
            table.push(IndexSet::default());
        }
        Transition {
            table,
            sol_next_index: IndexSet::default(),
            eol_next_index: IndexSet::default(),
        }
    }

    pub fn merge(&mut self, other: &Transition) {
        assert_eq!(self.table.len(), other.table.len());
        for i in 0..self.table.len() {
            self.table[i] = &self.table[i] | &other.table[i];
        }
        self.sol_next_index = &self.sol_next_index | &other.sol_next_index;
        self.eol_next_index = &self.eol_next_index | &other.eol_next_index;
    }
}

pub(crate) type IndexSet = BTreeSet<usize>;
