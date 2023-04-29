use super::inst::Inst;

pub(crate) struct Executer<'a> {
    insts: &'a Vec<Inst>,
    stack: Vec<(usize, usize, Vec<usize>, Vec<usize>)>,
    pc: usize,
    sp: usize,
    is_fail: bool,
    is_match: bool,
    check_result: bool,
    capture_needed: bool,
    cap_pos_start: Vec<usize>,
    cap_pos_end: Vec<usize>,
}

impl<'a> Executer<'a> {
    pub fn new(insts: &'a Vec<Inst>, capture_size: usize) -> Self {
        Executer {
            insts,
            stack: vec![],
            pc: 0,
            sp: 0,
            is_fail: false,
            is_match: false,
            check_result: false,
            capture_needed: true,
            cap_pos_start: vec![0; capture_size],
            cap_pos_end: vec![0; capture_size],
        }
    }

    fn reset(&mut self) {
        let insts = self.insts;
        let capture_needed = self.capture_needed;
        let capture_size = self.cap_pos_start.len();

        *self = Executer {
            insts,
            stack: vec![],
            pc: 0,
            sp: 0,
            is_fail: false,
            is_match: false,
            check_result: false,
            capture_needed,
            cap_pos_start: vec![0; capture_size],
            cap_pos_end: vec![0; capture_size],
        }
    }

    pub fn capture_mode(&mut self, need: bool) {
        self.capture_needed = need;
    }

    pub fn execute<'b>(&mut self, str: &'b str) -> Vec<&'b str> {
        for i in 0..str.len() {
            self.reset();
            self.sp = i;

            let result = self.execute_(str);
            if !result.is_empty() {
                return result;
            }
        }
        vec![]
    }

    fn execute_<'b>(&mut self, str: &'b str) -> Vec<&'b str> {
        loop {
            self.execute_step(str);

            if self.is_match {
                break; // match
            }
            if self.is_fail {
                if let Some((sp, pc, cap_s, cap_e)) = self.stack.pop() {
                    self.sp = sp;
                    self.pc = pc;
                    self.cap_pos_start = cap_s;
                    self.cap_pos_end = cap_e;
                    self.is_fail = false;
                } else {
                    return vec![]; // unmatch
                }
            }
        }

        let mut captures = vec![];
        captures.push(&str[self.cap_pos_start[0]..self.cap_pos_end[0]]);

        if self.capture_needed {
            for cap_id in 1..self.cap_pos_start.len() {
                let pos1 = self.cap_pos_start[cap_id];
                let pos2 = self.cap_pos_end[cap_id];

                if pos1 < pos2 {
                    captures.push(&str[pos1..pos2]);
                } else {
                    captures.push("");
                }
            }
        }

        return captures;
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
            Inst::CaptureStart(cap_id) => {
                self.cap_pos_start[*cap_id] = self.sp;
                self.pc += 1;
                return;
            }
            Inst::CaptureEnd(cap_id) => {
                self.cap_pos_end[*cap_id] = self.sp;
                self.pc += 1;
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
                self.stack.push((
                    self.sp,
                    self.pc.saturating_add_signed(*addr2),
                    self.cap_pos_start.clone(),
                    self.cap_pos_end.clone(),
                ));
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
