use self::{builder::Builder, matcher::Matcher};
use crate::parser::Parser;

mod builder;
mod matcher;

#[cfg(test)]
mod tests;

pub struct Nfa {
    pub(crate) nodes: Vec<Node>,
}

impl Nfa {
    pub fn new(pattern: &str) -> Result<Nfa, String> {
        let syntax = Parser::parse(pattern)?;
        let nodes = Builder::build(&syntax);

        Ok(Nfa { nodes })
    }

    pub fn is_match<'a>(&self, str: &'a str) -> Option<&'a str> {
        let mut matcher = Matcher::new(&self.nodes, 1);
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
