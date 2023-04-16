use super::dfa::Dfa;

pub(crate) struct Matcher<'a> {
    dfa: &'a Dfa,
    start_index: i32,
    last_index: i32,
}

impl<'a> Matcher<'a> {
    pub fn new(dfa: &'a Dfa) -> Self {
        Matcher {
            dfa,
            start_index: -1,
            last_index: -1,
        }
    }

    fn reset(&mut self) {
        *self = Matcher {
            dfa: self.dfa,
            start_index: -1,
            last_index: -1,
        }
    }

    pub fn execute<'b>(&mut self, str: &'b str) -> Option<&'b str> {
        for i in 0..str.len() {
            self.reset();

            let result = self.execute_(str, i);
            if result.is_some() {
                return result;
            }
        }
        None
    }

    fn execute_<'b>(&mut self, str: &'b str, sp: usize) -> Option<&'b str> {
        self.start_index = sp as i32;

        let mut index = self.dfa.indexmap.iter().find(|v| *v.1 == 0).unwrap().0;
        let mut sp = sp;

        while !index.is_empty() {
            let id = self.dfa.indexmap[index];
            let node = &self.dfa.nodes[id];

            if node.is_match {
                self.last_index = sp as i32;
            }

            if sp == 0 && !node.trans.sol_next_index.is_empty() {
                index = &node.trans.sol_next_index;
                continue;
            }
            if sp == str.len() && !node.trans.eol_next_index.is_empty() {
                index = &node.trans.eol_next_index;
                continue;
            }
            if sp >= str.len() {
                break; // end while loop
            }

            let c = str.chars().nth(sp).unwrap();
            index = &node.trans.table[c as usize];
            sp += 1;
        }

        if self.start_index <= self.last_index {
            let s = self.start_index as usize;
            let e = self.last_index as usize;
            Some(&str[s..e])
        } else {
            None
        }
    }
}
