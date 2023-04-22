use super::{EdgeAction, MatchSet, Node};

pub(crate) struct Matcher<'a> {
    nodes: &'a Vec<Node>,
    success_id: usize,
    capture_needed: bool,
    cap_starts: Vec<usize>,
    cap_ends: Vec<usize>,
}

impl<'a> Matcher<'a> {
    pub fn new(nodes: &'a Vec<Node>, success_id: usize, captuire_size: usize) -> Self {
        Matcher {
            nodes,
            success_id,
            capture_needed: true,
            cap_starts: vec![0; captuire_size],
            cap_ends: vec![0; captuire_size],
        }
    }

    fn reset(&mut self) {
        let captuire_size = self.cap_starts.len();

        *self = Matcher {
            nodes: self.nodes,
            success_id: self.success_id,
            capture_needed: self.capture_needed,
            cap_starts: vec![0; captuire_size],
            cap_ends: vec![0; captuire_size],
        }
    }

    pub fn capture_mode(&mut self, need: bool) {
        self.capture_needed = need;
    }

    pub fn execute<'b>(&mut self, str: &'b str) -> Vec<&'b str> {
        for i in 0..str.len() {
            self.reset();

            let result = self.execute_(str, i, 0);
            if result.is_some() {
                let mut captures = vec![];
                captures.push(&str[self.cap_starts[0]..self.cap_ends[0]]);

                if self.capture_needed {
                    for cap_id in 1..self.cap_starts.len() {
                        let start = self.cap_starts[cap_id];
                        let end = self.cap_ends[cap_id];

                        if start < end {
                            captures.push(&str[start..end]);
                        } else {
                            captures.push("");
                        }
                    }
                }
                return captures;
            }
        }

        vec![] // unmatch
    }

    fn execute_<'b>(&mut self, str: &'b str, sp: usize, id: usize) -> Option<usize> {
        if id == self.success_id {
            return Some(id);
        }

        let node = &self.nodes[id];
        for edge in node.nexts.iter() {
            #[rustfmt::skip]
            let result = match &edge.action {
                EdgeAction::Asap =>
                    self.execute_(str, sp, edge.next_id),
                EdgeAction::CaptureStart(cap_id) => {
                    let old_sp = self.cap_starts[*cap_id];
                    self.cap_starts[*cap_id] = sp;

                    let result = self.execute_(str, sp, edge.next_id);
                    if result.is_none() {
                        self.cap_starts[*cap_id] = old_sp;
                    }
                    result
                },
                EdgeAction::CaptureEnd(cap_id) => {
                    let old_sp = self.cap_ends[*cap_id];
                    self.cap_ends[*cap_id] = sp;

                    let result = self.execute_(str, sp, edge.next_id);
                    if result.is_none() {
                        self.cap_ends[*cap_id] = old_sp;
                    }
                    result
                },
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
