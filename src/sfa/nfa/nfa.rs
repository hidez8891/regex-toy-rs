use crate::parser::Parser;

use super::{builder::Builder, matcher::Matcher};

pub struct Nfa {
    pub nodes: Vec<Node>,
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

pub struct Node {
    pub nexts: Vec<Edge>,
}

pub struct Edge {
    pub action: EdgeAction,
    pub next_id: usize,
    pub is_greedy: bool,
}

pub enum EdgeAction {
    Asap,
    Match(char),
    MatchAny,
    MatchSOL,
    MatchEOL,
    MatchIncludeSet(Vec<MatchSet>),
    MatchExcludeSet(Vec<MatchSet>),
}

pub enum MatchSet {
    Char(char),
    Range(char, char),
}
