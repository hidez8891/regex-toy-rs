use self::{builder::Builder, matcher::Matcher};
use crate::parser::Parser;

mod builder;
mod matcher;

#[cfg(test)]
mod tests;

pub struct Nfa {
    pub(crate) nodes: Vec<Node>,
    pub(crate) capture_size: usize,
}

impl Nfa {
    pub fn new(pattern: &str) -> Result<Nfa, String> {
        let syntax = Parser::parse(pattern)?;
        let (nodes, capture_size) = Builder::build(&syntax);

        Ok(Nfa {
            nodes,
            capture_size,
        })
    }

    pub fn is_match<'a>(&self, str: &'a str) -> bool {
        let mut matcher = Matcher::new(&self.nodes, 1, self.capture_size);
        matcher.capture_mode(false);
        !matcher.execute(str).is_empty()
    }

    pub fn captures<'a>(&self, str: &'a str) -> Vec<&'a str> {
        let mut matcher = Matcher::new(&self.nodes, 1, self.capture_size);
        matcher.capture_mode(true);
        matcher.execute(str)
    }
}

pub(crate) struct Node {
    pub nexts: Vec<Edge>,
}

pub(crate) struct Edge {
    pub action: EdgeAction,
    pub next_id: usize,
    pub is_greedy: bool,
}

pub(crate) enum EdgeAction {
    Asap,
    CaptureStart(usize),
    CaptureEnd(usize),
    Match(char),
    MatchAny,
    MatchSOL,
    MatchEOL,
    MatchIncludeSet(Vec<MatchSet>),
    MatchExcludeSet(Vec<MatchSet>),
}

pub(crate) enum MatchSet {
    Char(char),
    Range(char, char),
}
