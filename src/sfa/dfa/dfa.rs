use std::collections::{BTreeSet, HashMap};

use super::{builder::Builder, matcher::Matcher};
use crate::sfa::Nfa;

pub struct Dfa {
    pub(crate) nodes: Vec<Node>,
    pub(crate) nodemap: HashMap<IndexSet, usize>,
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
    pub trans: Vec<IndexSet>,
    pub start_line: IndexSet,
    pub end_line: IndexSet,
}

impl Transition {
    pub fn new(size: usize) -> Self {
        let mut trans = Vec::with_capacity(size);
        for _ in 0..size {
            trans.push(IndexSet::default());
        }
        Transition {
            trans,
            start_line: IndexSet::default(),
            end_line: IndexSet::default(),
        }
    }

    pub fn merge(&mut self, other: &Transition) {
        assert_eq!(self.trans.len(), other.trans.len());
        for i in 0..self.trans.len() {
            self.trans[i] = &self.trans[i] | &other.trans[i];
        }
        self.start_line = &self.start_line | &other.start_line;
        self.end_line = &self.end_line | &other.end_line;
    }
}

pub(crate) type IndexSet = BTreeSet<usize>;
