use super::inst::Inst;

pub(crate) struct Executer<'a> {
    insts: &'a Vec<Inst>,
    stack: Vec<(usize, usize)>,
    pc: usize,
    sp: usize,
    is_fail: bool,
    is_match: bool,
    check_result: bool,
}

impl<'a> Executer<'a> {
    pub fn new(insts: &'a Vec<Inst>) -> Self {
        Executer {
            insts,
            stack: vec![],
            pc: 0,
            sp: 0,
            is_fail: false,
            is_match: false,
            check_result: false,
        }
    }

    fn reset(&mut self) {
        let insts = self.insts;
        *self = Executer {
            insts,
            stack: vec![],
            pc: 0,
            sp: 0,
            is_fail: false,
            is_match: false,
            check_result: false,
        }
    }

    pub fn execute<'b>(&mut self, str: &'b str) -> Option<&'b str> {
        for i in 0..str.len() {
            self.reset();
            self.sp = i;

            let result = self.execute_(str);
            if result.is_some() {
                return result;
            }
        }
        None
    }

    fn execute_<'b>(&mut self, str: &'b str) -> Option<&'b str> {
        let start_index = self.sp;

        loop {
            self.execute_step(str);

            if self.is_match {
                return Some(&str[start_index..self.sp]);
            }
            if self.is_fail {
                if let Some((sp, pc)) = self.stack.pop() {
                    self.sp = sp;
                    self.pc = pc;
                    self.is_fail = false;
                } else {
                    return None; // unmatch
                }
            }
        }
    }

    fn execute_step(&mut self, str: &str) {
        match &self.insts[self.pc] {
            Inst::Fail => {
                self.is_fail = true;
                return;
            }
            Inst::Success => {
                self.is_match = true;
                return;
            }
            Inst::Seek(offset) => {
                self.sp = self.sp.saturating_add_signed(*offset);
                self.pc += 1;
                return;
            }
            Inst::Jmp(addr) => {
                self.pc = self.pc.saturating_add_signed(*addr);
                return;
            }
            Inst::JmpIfTrue(addr) => {
                if self.check_result {
                    self.pc = self.pc.saturating_add_signed(*addr);
                } else {
                    self.pc += 1;
                }
                return;
            }
            Inst::JmpIfFalse(addr) => {
                if !self.check_result {
                    self.pc = self.pc.saturating_add_signed(*addr);
                } else {
                    self.pc += 1;
                }
                return;
            }
            Inst::Split(addr1, addr2) => {
                self.stack
                    .push((self.sp, self.pc.saturating_add_signed(*addr2)));
                self.pc = self.pc.saturating_add_signed(*addr1);
                return;
            }
            Inst::MatchChar(s) => {
                if let Some(c) = str.chars().nth(self.sp) {
                    if *s == c {
                        self.sp += 1;
                        self.pc += 1;
                        return;
                    }
                }
            }
            Inst::MatchCharAny => {
                if let Some(_) = str.chars().nth(self.sp) {
                    self.sp += 1;
                    self.pc += 1;
                    return;
                }
            }
            Inst::MatchPosSOL => {
                if self.sp == 0 {
                    self.pc += 1;
                    return;
                }
            }
            Inst::MatchPosEOL => {
                if self.sp == str.len() {
                    self.pc += 1;
                    return;
                }
            }
            Inst::CheckInclude(a, b) => {
                if let Some(c) = str.chars().nth(self.sp) {
                    self.check_result = *a <= c && c <= *b;
                    self.pc += 1;
                    return;
                }
            }
            Inst::CheckExclude(a, b) => {
                if let Some(c) = str.chars().nth(self.sp) {
                    self.check_result = *a > c || c > *b;
                    self.pc += 1;
                    return;
                }
            }
        }

        // unmatch
        self.is_fail = true;
    }
}
