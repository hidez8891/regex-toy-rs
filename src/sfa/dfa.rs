use std::collections::{BTreeSet, HashMap};

use self::{builder::Builder, matcher::Matcher};
use crate::sfa::Nfa;

mod builder;
mod matcher;

#[cfg(test)]
mod tests;

pub struct Dfa {
    nodes: Vec<Node>,
    indexmap: HashMap<IndexSet, usize>,
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

struct Node {
    trans: Transition,
    is_match: bool,
}

struct Transition {
    table: Vec<IndexSet>,
    sol_next_index: IndexSet,
    eol_next_index: IndexSet,
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

type IndexSet = BTreeSet<usize>;
