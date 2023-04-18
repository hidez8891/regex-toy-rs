use super::{EdgeAction, MatchSet, Node};

pub(crate) struct Matcher<'a> {
    nodes: &'a Vec<Node>,
    success_id: usize,
    start_index: usize,
}

impl<'a> Matcher<'a> {
    pub fn new(nodes: &'a Vec<Node>, success_id: usize) -> Self {
        Matcher {
            nodes,
            success_id,
            start_index: 0,
        }
    }

    fn reset(&mut self) {
        *self = Matcher {
            nodes: self.nodes,
            success_id: self.success_id,
            start_index: 0,
        }
    }

    pub fn execute<'b>(&mut self, str: &'b str) -> Option<&'b str> {
        for i in 0..str.len() {
            self.reset();
            self.start_index = i;

            let result = self.execute_(str, i, 0);
            if result.is_some() {
                return result;
            }
        }
        None
    }

    fn execute_<'b>(&self, str: &'b str, sp: usize, id: usize) -> Option<&'b str> {
        if id == self.success_id {
            return Some(&str[self.start_index..sp]);
        }

        let node = &self.nodes[id];
        for edge in node.nexts.iter() {
            #[rustfmt::skip]
            let result = match &edge.action {
                EdgeAction::Asap =>
                    self.execute_(str, sp, edge.next_id),
                EdgeAction::Match(t) =>
                    str
                    .chars()
                    .nth(sp)
                    .filter(|c| *c == *t)
                    .and_then(|_| self.execute_(str, sp + 1, edge.next_id)),
                EdgeAction::MatchAny =>
                    str
                    .chars()
                    .nth(sp)
                    .and_then(|_| self.execute_(str, sp + 1, edge.next_id)),
                EdgeAction::MatchSOL =>
                    Some(sp)
                    .filter(|p| *p == 0)
                    .and_then(|_| self.execute_(str, sp, edge.next_id)),
                EdgeAction::MatchEOL =>
                    Some(sp)
                    .filter(|p| *p == str.len())
                    .and_then(|_| self.execute_(str, sp, edge.next_id)),
                EdgeAction::MatchIncludeSet(set) =>
                    str
                    .chars()
                    .nth(sp)
                    .filter(|c| {
                        set.iter().any(|m| match m {
                            MatchSet::Char(t) => *t == *c,
                            MatchSet::Range(a, b) => *a <= *c && *c <= *b,
                        })
                    })
                    .and_then(|_| self.execute_(str, sp + 1, edge.next_id)),
                EdgeAction::MatchExcludeSet(set) =>
                    str
                    .chars()
                    .nth(sp)
                    .filter(|c| {
                        set.iter().all(|m| match m {
                            MatchSet::Char(t) => *t != *c,
                            MatchSet::Range(a, b) => *c < *a || *b < *c,
                        })
                    })
                    .and_then(|_| self.execute_(str, sp + 1, edge.next_id)),
            };

            if result.is_some() {
                return result;
            }
        }

        None
    }
}
