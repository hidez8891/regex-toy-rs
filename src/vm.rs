use crate::parser::{MatchKind, Parser, SyntaxKind, SyntaxNode};

pub struct VM {
    insts: Vec<Inst>,
}

impl VM {
    pub fn new(pattern: &str) -> Result<VM, String> {
        let syntax = Parser::new(pattern)?;
        let insts = Compiler::compile(&syntax);
        Ok(VM { insts })
    }

    pub fn is_match<'a>(&self, str: &'a str) -> Option<&'a str> {
        Executer::is_match(&self.insts, str)
    }

    #[cfg(test)]
    pub fn dump(&self) {
        for inst in self.insts.iter() {
            println!("{:?}", inst);
        }
    }
}

#[derive(Debug)]
enum Inst {
    Fail,
    Match,
    Jmp(isize),
    Split(isize, isize),
    Char(char),
    Any,
    PosSOL,
    PosEOL,
    Include(char, char),
    Exclude(char, char),
}

struct Compiler {}

impl Compiler {
    fn compile(syntax: &SyntaxNode) -> Vec<Inst> {
        let compiler = Compiler {};
        let mut insts = compiler.compile_root(syntax);
        insts.push(Inst::Match);
        insts
    }

    fn compile_root(&self, syntax: &SyntaxNode) -> Vec<Inst> {
        match &syntax.kind {
            SyntaxKind::Group => self.compile_group(syntax),
            SyntaxKind::Union => self.compile_union(syntax),
            SyntaxKind::Match(kind) => match kind {
                MatchKind::Any => self.compile_match_any(),
                MatchKind::Char(c) => self.compile_match_char(*c),
                MatchKind::Range(_, _) => unreachable!(),
            },
            SyntaxKind::None => unreachable!(),
            _ => todo!(),
        }
    }

    fn compile_group(&self, syntax: &SyntaxNode) -> Vec<Inst> {
        let mut insts = Vec::new();
        for child in syntax.children.iter() {
            insts.extend(self.compile_root(child));
        }
        insts
    }

    fn compile_union(&self, syntax: &SyntaxNode) -> Vec<Inst> {
        let mut insts = Vec::new();

        let mut dst_addr = 2;
        insts.push(Inst::Fail);

        for child in syntax.children.iter().rev() {
            let mut child_insts = self.compile_root(child);
            let next_addr = child_insts.len() as isize + 2;

            insts.push(Inst::Jmp(dst_addr));
            child_insts.reverse();
            insts.extend(child_insts);
            insts.push(Inst::Split(1, next_addr));

            dst_addr += next_addr;
        }

        insts.reverse();
        insts
    }

    fn compile_match_any(&self) -> Vec<Inst> {
        [Inst::Any].into()
    }

    fn compile_match_char(&self, c: char) -> Vec<Inst> {
        [Inst::Char(c)].into()
    }
}

struct Executer<'a, 'b> {
    insts: &'a Vec<Inst>,
    str: &'b str,
    stack: Vec<(usize, usize)>,
    ic: usize,
    pc: usize,
    is_fail: bool,
    is_match: bool,
}

impl<'a, 'b> Executer<'a, 'b> {
    fn is_match(insts: &'a Vec<Inst>, str: &'b str) -> Option<&'b str> {
        for i in 0..str.len() {
            let mut exec = Executer {
                insts,
                str,
                stack: vec![],
                ic: i,
                pc: 0,
                is_fail: false,
                is_match: false,
            };

            let result = exec.execute();
            if result.is_some() {
                return result;
            }
        }
        None
    }

    fn execute(&mut self) -> Option<&'b str> {
        let start_index = self.ic;

        loop {
            self.execute_step();

            if self.is_match {
                return Some(&self.str[start_index..self.ic]);
            }
            if self.is_fail {
                if let Some((s_ic, s_pc)) = self.stack.pop() {
                    self.ic = s_ic;
                    self.pc = s_pc;
                    self.is_fail = false;
                } else {
                    return None; // unmatch
                }
            }
        }
    }

    fn execute_step(&mut self) {
        match &self.insts[self.pc] {
            Inst::Fail => {
                self.is_fail = true;
                return;
            }
            Inst::Match => {
                self.is_match = true;
                return;
            }
            Inst::Jmp(addr) => {
                self.pc = self.pc.saturating_add_signed(*addr);
                return;
            }
            Inst::Split(addr1, addr2) => {
                self.stack
                    .push((self.ic, self.pc.saturating_add_signed(*addr2)));
                self.pc = self.pc.saturating_add_signed(*addr1);
                return;
            }
            Inst::Char(s) => {
                if let Some(c) = self.str.chars().nth(self.ic) {
                    if *s == c {
                        self.ic += 1;
                        self.pc += 1;
                        return;
                    }
                }
            }
            Inst::Any => {
                if let Some(_) = self.str.chars().nth(self.ic) {
                    self.ic += 1;
                    self.pc += 1;
                    return;
                }
            }
            Inst::PosSOL => {
                if self.ic == 0 {
                    self.pc += 1;
                    return;
                }
            }
            Inst::PosEOL => {
                if self.ic == self.str.len() {
                    self.pc += 1;
                    return;
                }
            }
            Inst::Include(a, b) => {
                if let Some(c) = self.str.chars().nth(self.ic) {
                    if *a <= c && c <= *b {
                        self.ic += 1;
                        self.pc += 1;
                        return;
                    }
                }
            }
            Inst::Exclude(a, b) => {
                if let Some(c) = self.str.chars().nth(self.ic) {
                    if *a > c || c > *b {
                        self.ic += 1;
                        self.pc += 1;
                        return;
                    }
                }
            }
        }

        // unmatch
        self.is_fail = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(pattern: &str) -> VM {
        VM::new(pattern).unwrap()
    }

    #[cfg(test)]
    mod basic_match {
        use super::*;

        #[test]
        fn match_char() {
            let src = "abc";
            let vm = run(src);

            assert_eq!(vm.is_match("abc"), Some("abc"));
            assert_eq!(vm.is_match("ab"), None);
            assert_eq!(vm.is_match("abcd"), Some("abc"));
            assert_eq!(vm.is_match("zabc"), Some("abc"));
        }

        #[test]
        fn match_metachar() {
            let src = r"a\+c";
            let vm = run(src);

            assert_eq!(vm.is_match("a+c"), Some("a+c"));
            assert_eq!(vm.is_match("aac"), None);
            assert_eq!(vm.is_match("ac"), None);
            assert_eq!(vm.is_match("a+cz"), Some("a+c"));
            assert_eq!(vm.is_match("za+c"), Some("a+c"));
        }

        #[test]
        fn match_any() {
            {
                let src = "a.c";
                let vm = run(src);

                assert_eq!(vm.is_match("abc"), Some("abc"));
                assert_eq!(vm.is_match("adc"), Some("adc"));
                assert_eq!(vm.is_match("ac"), None);
                assert_eq!(vm.is_match("abbc"), None);
                assert_eq!(vm.is_match("zabc"), Some("abc"));
                assert_eq!(vm.is_match("abcz"), Some("abc"));
            }
            {
                let src = "a.";
                let vm = run(src);

                assert_eq!(vm.is_match("ab"), Some("ab"));
                assert_eq!(vm.is_match("ad"), Some("ad"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("abz"), Some("ab"));
                assert_eq!(vm.is_match("zab"), Some("ab"));
            }
        }

        #[test]
        fn match_sol() {
            {
                let src = "^abc";
                let vm = run(src);

                assert_eq!(vm.is_match("abc"), Some("abc"));
                assert_eq!(vm.is_match("zabc"), None);
                assert_eq!(vm.is_match("abcz"), Some("abc"));
            }
        }

        #[test]
        fn match_eol() {
            {
                let src = "abc$";
                let vm = run(src);

                assert_eq!(vm.is_match("abc"), Some("abc"));
                assert_eq!(vm.is_match("zabc"), Some("abc"));
                assert_eq!(vm.is_match("abcz"), None);
            }
        }
    }

    #[test]
    fn group() {
        {
            let src = "a(bc)d";
            let vm = run(src);

            assert_eq!(vm.is_match("abcd"), Some("abcd"));
            assert_eq!(vm.is_match("abc"), None);
            assert_eq!(vm.is_match("ad"), None);
            assert_eq!(vm.is_match("zabcd"), Some("abcd"));
            assert_eq!(vm.is_match("abcdz"), Some("abcd"));
        }
        {
            let src = "a(bc)";
            let vm = run(src);

            assert_eq!(vm.is_match("abc"), Some("abc"));
            assert_eq!(vm.is_match("a"), None);
            assert_eq!(vm.is_match("zabc"), Some("abc"));
            assert_eq!(vm.is_match("abcd"), Some("abc"));
        }
    }

    #[test]
    fn union() {
        let src = "abc|def|ghi";
        let vm = run(src);

        assert_eq!(vm.is_match("abc"), Some("abc"));
        assert_eq!(vm.is_match("def"), Some("def"));
        assert_eq!(vm.is_match("ghi"), Some("ghi"));
        assert_eq!(vm.is_match("adg"), None);
        assert_eq!(vm.is_match("ab"), None);
        assert_eq!(vm.is_match("zabc"), Some("abc"));
        assert_eq!(vm.is_match("defz"), Some("def"));
    }

    #[cfg(test)]
    mod longest {
        use super::*;

        #[test]
        fn star() {
            {
                let src = "ab*c";
                let vm = run(src);

                assert_eq!(vm.is_match("ac"), Some("ac"));
                assert_eq!(vm.is_match("abc"), Some("abc"));
                assert_eq!(vm.is_match("abbc"), Some("abbc"));
                assert_eq!(vm.is_match("abbbc"), Some("abbbc"));
                assert_eq!(vm.is_match("az"), None);
                assert_eq!(vm.is_match("zac"), Some("ac"));
                assert_eq!(vm.is_match("acz"), Some("ac"));
            }
            {
                let src = "ab*";
                let vm = run(src);

                assert_eq!(vm.is_match("a"), Some("a"));
                assert_eq!(vm.is_match("ab"), Some("ab"));
                assert_eq!(vm.is_match("abb"), Some("abb"));
                assert_eq!(vm.is_match("abbb"), Some("abbb"));
                assert_eq!(vm.is_match("b"), None);
                assert_eq!(vm.is_match("za"), Some("a"));
                assert_eq!(vm.is_match("az"), Some("a"));
            }
            {
                let src = "ab*b*";
                let vm = run(src);

                assert_eq!(vm.is_match("a"), Some("a"));
                assert_eq!(vm.is_match("ab"), Some("ab"));
                assert_eq!(vm.is_match("abb"), Some("abb"));
                assert_eq!(vm.is_match("abbb"), Some("abbb"));
                assert_eq!(vm.is_match("b"), None);
                assert_eq!(vm.is_match("za"), Some("a"));
                assert_eq!(vm.is_match("az"), Some("a"));
            }
            {
                let src = "a.*b";
                let vm = run(src);

                assert_eq!(vm.is_match("ab"), Some("ab"));
                assert_eq!(vm.is_match("axb"), Some("axb"));
                assert_eq!(vm.is_match("axbaxb"), Some("axbaxb"));
                #[rustfmt::skip]
            assert_eq!(vm.is_match("axaxbxb"), Some("axaxbxb"));
                assert_eq!(vm.is_match("baxb"), Some("axb"));
                assert_eq!(vm.is_match("axbz"), Some("axb"));
            }
        }

        #[test]
        fn plus() {
            {
                let src = "ab+c";
                let vm = run(src);

                assert_eq!(vm.is_match("abc"), Some("abc"));
                assert_eq!(vm.is_match("abbc"), Some("abbc"));
                assert_eq!(vm.is_match("abbbc"), Some("abbbc"));
                assert_eq!(vm.is_match("ac"), None);
                assert_eq!(vm.is_match("zabc"), Some("abc"));
                assert_eq!(vm.is_match("abcz"), Some("abc"));
            }
            {
                let src = "ab+";
                let vm = run(src);

                assert_eq!(vm.is_match("ab"), Some("ab"));
                assert_eq!(vm.is_match("abb"), Some("abb"));
                assert_eq!(vm.is_match("abbb"), Some("abbb"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("zab"), Some("ab"));
                assert_eq!(vm.is_match("abz"), Some("ab"));
            }
            {
                let src = "ab+b+";
                let vm = run(src);

                assert_eq!(vm.is_match("abb"), Some("abb"));
                assert_eq!(vm.is_match("abbb"), Some("abbb"));
                assert_eq!(vm.is_match("abbbb"), Some("abbbb"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("ab"), None);
                assert_eq!(vm.is_match("zabb"), Some("abb"));
                assert_eq!(vm.is_match("abbz"), Some("abb"));
            }
            {
                let src = "a.+b";
                let vm = run(src);

                assert_eq!(vm.is_match("ab"), None);
                assert_eq!(vm.is_match("axb"), Some("axb"));
                assert_eq!(vm.is_match("axbaxb"), Some("axbaxb"));
                assert_eq!(vm.is_match("axaxbxb"), Some("axaxbxb"));
                assert_eq!(vm.is_match("baxb"), Some("axb"));
                assert_eq!(vm.is_match("axbz"), Some("axb"));
            }
        }

        #[test]
        fn option() {
            {
                let src = "ab?c";
                let vm = run(src);

                assert_eq!(vm.is_match("ac"), Some("ac"));
                assert_eq!(vm.is_match("abc"), Some("abc"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("zac"), Some("ac"));
                assert_eq!(vm.is_match("acz"), Some("ac"));
            }
            {
                let src = "ab?";
                let vm = run(src);

                assert_eq!(vm.is_match("a"), Some("a"));
                assert_eq!(vm.is_match("ab"), Some("ab"));
                assert_eq!(vm.is_match("b"), None);
                assert_eq!(vm.is_match("za"), Some("a"));
                assert_eq!(vm.is_match("az"), Some("a"));
            }
        }

        #[test]
        fn repeat() {
            {
                let src = "a{3}";
                let vm = run(src);

                assert_eq!(vm.is_match("aaa"), Some("aaa"));
                assert_eq!(vm.is_match("aaaaa"), Some("aaa"));
                assert_eq!(vm.is_match("aa"), None);
                assert_eq!(vm.is_match("zaaa"), Some("aaa"));
                assert_eq!(vm.is_match("aaaz"), Some("aaa"));
            }
            {
                let src = "abc{3}";
                let vm = run(src);

                assert_eq!(vm.is_match("abccc"), Some("abccc"));
                assert_eq!(vm.is_match("abccccc"), Some("abccc"));
                assert_eq!(vm.is_match("abc"), None);
                assert_eq!(vm.is_match("zabccc"), Some("abccc"));
                assert_eq!(vm.is_match("abcccz"), Some("abccc"));
            }
            {
                let src = "(abc){3}";
                let vm = run(src);

                assert_eq!(vm.is_match("abcabcabc"), Some("abcabcabc"));
                assert_eq!(vm.is_match("abcabc"), None);
                assert_eq!(vm.is_match("zabcabcabc"), Some("abcabcabc"));
                assert_eq!(vm.is_match("abcabcabcz"), Some("abcabcabc"));
            }
        }

        #[test]
        fn repeat_min() {
            {
                let src = "a{2,}";
                let vm = run(src);

                assert_eq!(vm.is_match("aa"), Some("aa"));
                assert_eq!(vm.is_match("aaa"), Some("aaa"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("zaaa"), Some("aaa"));
                assert_eq!(vm.is_match("aaaz"), Some("aaa"));
            }
            {
                let src = "abc{2,}";
                let vm = run(src);

                assert_eq!(vm.is_match("abcc"), Some("abcc"));
                assert_eq!(vm.is_match("abccc"), Some("abccc"));
                assert_eq!(vm.is_match("abc"), None);
                assert_eq!(vm.is_match("zabcc"), Some("abcc"));
                assert_eq!(vm.is_match("abccz"), Some("abcc"));
            }
            {
                let src = "(abc){2,}";
                let vm = run(src);

                assert_eq!(vm.is_match("abcabc"), Some("abcabc"));
                assert_eq!(vm.is_match("abcabcabc"), Some("abcabcabc"));
                assert_eq!(vm.is_match("abc"), None);
                assert_eq!(vm.is_match("zabcabc"), Some("abcabc"));
                assert_eq!(vm.is_match("abcabcz"), Some("abcabc"));
            }
        }

        #[test]
        fn repeat_range() {
            {
                let src = "a{2,3}";
                let vm = run(src);

                assert_eq!(vm.is_match("aa"), Some("aa"));
                assert_eq!(vm.is_match("aaa"), Some("aaa"));
                assert_eq!(vm.is_match("aaaa"), Some("aaa"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("zaa"), Some("aa"));
                assert_eq!(vm.is_match("aaz"), Some("aa"));
            }
            {
                let src = "abc{2,3}";
                let vm = run(src);

                assert_eq!(vm.is_match("abcc"), Some("abcc"));
                assert_eq!(vm.is_match("abccc"), Some("abccc"));
                assert_eq!(vm.is_match("abcccc"), Some("abccc"));
                assert_eq!(vm.is_match("abc"), None);
                assert_eq!(vm.is_match("zabcc"), Some("abcc"));
                assert_eq!(vm.is_match("abccz"), Some("abcc"));
            }
            {
                let src = "(abc){2,3}";
                let vm = run(src);

                assert_eq!(vm.is_match("abcabc"), Some("abcabc"));
                assert_eq!(vm.is_match("abcabcabc"), Some("abcabcabc"));
                assert_eq!(vm.is_match("abcabcabcabc"), Some("abcabcabc"));
                assert_eq!(vm.is_match("abc"), None);
                assert_eq!(vm.is_match("zabcabc"), Some("abcabc"));
                assert_eq!(vm.is_match("abcabcz"), Some("abcabc"));
            }
        }
    }

    #[cfg(test)]
    mod shortest {
        use super::*;

        #[test]
        fn star() {
            {
                let src = "ab*?c";
                let vm = run(src);

                assert_eq!(vm.is_match("ac"), Some("ac"));
                assert_eq!(vm.is_match("abc"), Some("abc"));
                assert_eq!(vm.is_match("abbc"), Some("abbc"));
                assert_eq!(vm.is_match("abbbc"), Some("abbbc"));
                assert_eq!(vm.is_match("az"), None);
                assert_eq!(vm.is_match("zac"), Some("ac"));
                assert_eq!(vm.is_match("acz"), Some("ac"));
            }
            {
                let src = "ab*?";
                let vm = run(src);

                assert_eq!(vm.is_match("a"), Some("a"));
                assert_eq!(vm.is_match("ab"), Some("a"));
                assert_eq!(vm.is_match("abb"), Some("a"));
                assert_eq!(vm.is_match("abbb"), Some("a"));
                assert_eq!(vm.is_match("b"), None);
                assert_eq!(vm.is_match("za"), Some("a"));
                assert_eq!(vm.is_match("az"), Some("a"));
            }
            {
                let src = "ab*?b*?";
                let vm = run(src);

                assert_eq!(vm.is_match("a"), Some("a"));
                assert_eq!(vm.is_match("ab"), Some("a"));
                assert_eq!(vm.is_match("abb"), Some("a"));
                assert_eq!(vm.is_match("abbb"), Some("a"));
                assert_eq!(vm.is_match("b"), None);
                assert_eq!(vm.is_match("za"), Some("a"));
                assert_eq!(vm.is_match("az"), Some("a"));
            }
            {
                let src = "a.*?b";
                let vm = run(src);

                assert_eq!(vm.is_match("ab"), Some("ab"));
                assert_eq!(vm.is_match("axb"), Some("axb"));
                assert_eq!(vm.is_match("axbaxb"), Some("axb"));
                #[rustfmt::skip]
            assert_eq!(vm.is_match("axaxbxb"), Some("axaxb"));
                assert_eq!(vm.is_match("baxb"), Some("axb"));
                assert_eq!(vm.is_match("axbz"), Some("axb"));
            }
        }

        #[test]
        fn plus() {
            {
                let src = "ab+?c";
                let vm = run(src);

                assert_eq!(vm.is_match("abc"), Some("abc"));
                assert_eq!(vm.is_match("abbc"), Some("abbc"));
                assert_eq!(vm.is_match("abbbc"), Some("abbbc"));
                assert_eq!(vm.is_match("ac"), None);
                assert_eq!(vm.is_match("zabc"), Some("abc"));
                assert_eq!(vm.is_match("abcz"), Some("abc"));
            }
            {
                let src = "ab+?";
                let vm = run(src);

                assert_eq!(vm.is_match("ab"), Some("ab"));
                assert_eq!(vm.is_match("abb"), Some("ab"));
                assert_eq!(vm.is_match("abbb"), Some("ab"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("zab"), Some("ab"));
                assert_eq!(vm.is_match("abz"), Some("ab"));
            }
            {
                let src = "ab+?b+?";
                let vm = run(src);

                assert_eq!(vm.is_match("abb"), Some("abb"));
                assert_eq!(vm.is_match("abbb"), Some("abb"));
                assert_eq!(vm.is_match("abbbb"), Some("abb"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("ab"), None);
                assert_eq!(vm.is_match("zabb"), Some("abb"));
                assert_eq!(vm.is_match("abbz"), Some("abb"));
            }
            {
                let src = "a.+?b";
                let vm = run(src);

                assert_eq!(vm.is_match("ab"), None);
                assert_eq!(vm.is_match("axb"), Some("axb"));
                assert_eq!(vm.is_match("axbaxb"), Some("axb"));
                assert_eq!(vm.is_match("axaxbxb"), Some("axaxb"));
                assert_eq!(vm.is_match("baxb"), Some("axb"));
                assert_eq!(vm.is_match("axbz"), Some("axb"));
            }
        }

        #[test]
        fn option() {
            {
                let src = "ab??c";
                let vm = run(src);

                assert_eq!(vm.is_match("ac"), Some("ac"));
                assert_eq!(vm.is_match("abc"), Some("abc"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("zac"), Some("ac"));
                assert_eq!(vm.is_match("acz"), Some("ac"));
            }
            {
                let src = "ab??";
                let vm = run(src);

                assert_eq!(vm.is_match("a"), Some("a"));
                assert_eq!(vm.is_match("ab"), Some("a"));
                assert_eq!(vm.is_match("b"), None);
                assert_eq!(vm.is_match("za"), Some("a"));
                assert_eq!(vm.is_match("az"), Some("a"));
            }
        }

        #[test]
        fn repeat() {
            {
                let src = "a{3}?";
                let _ = run(src);

                // show warning error
            }
        }

        #[test]
        fn repeat_min() {
            {
                let src = "a{2,}?";
                let vm = run(src);

                assert_eq!(vm.is_match("aa"), Some("aa"));
                assert_eq!(vm.is_match("aaa"), Some("aa"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("zaaa"), Some("aa"));
                assert_eq!(vm.is_match("aaaz"), Some("aa"));
            }
            {
                let src = "abc{2,}?";
                let vm = run(src);

                assert_eq!(vm.is_match("abcc"), Some("abcc"));
                assert_eq!(vm.is_match("abccc"), Some("abcc"));
                assert_eq!(vm.is_match("abc"), None);
                assert_eq!(vm.is_match("zabcc"), Some("abcc"));
                assert_eq!(vm.is_match("abccz"), Some("abcc"));
            }
            {
                let src = "(abc){2,}?";
                let vm = run(src);

                assert_eq!(vm.is_match("abcabc"), Some("abcabc"));
                assert_eq!(vm.is_match("abcabcabc"), Some("abcabc"));
                assert_eq!(vm.is_match("abc"), None);
                assert_eq!(vm.is_match("zabcabc"), Some("abcabc"));
                assert_eq!(vm.is_match("abcabcz"), Some("abcabc"));
            }
        }

        #[test]
        fn repeat_range() {
            {
                let src = "a{2,3}?";
                let vm = run(src);

                assert_eq!(vm.is_match("aa"), Some("aa"));
                assert_eq!(vm.is_match("aaa"), Some("aa"));
                assert_eq!(vm.is_match("aaaa"), Some("aa"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("zaa"), Some("aa"));
                assert_eq!(vm.is_match("aaz"), Some("aa"));
            }
            {
                let src = "abc{2,3}?";
                let vm = run(src);

                assert_eq!(vm.is_match("abcc"), Some("abcc"));
                assert_eq!(vm.is_match("abccc"), Some("abcc"));
                assert_eq!(vm.is_match("abcccc"), Some("abcc"));
                assert_eq!(vm.is_match("abc"), None);
                assert_eq!(vm.is_match("zabcc"), Some("abcc"));
                assert_eq!(vm.is_match("abccz"), Some("abcc"));
            }
            {
                let src = "(abc){2,3}?";
                let vm = run(src);

                assert_eq!(vm.is_match("abcabc"), Some("abcabc"));
                assert_eq!(vm.is_match("abcabcabc"), Some("abcabc"));
                assert_eq!(vm.is_match("abcabcabcabc"), Some("abcabc"));
                assert_eq!(vm.is_match("abc"), None);
                assert_eq!(vm.is_match("zabcabc"), Some("abcabc"));
                assert_eq!(vm.is_match("abcabcz"), Some("abcabc"));
            }
        }
    }

    #[cfg(test)]
    mod set {
        use super::*;

        #[test]
        fn positive() {
            {
                let src = "a[b-z]d";
                let vm = run(src);

                assert_eq!(vm.is_match("abd"), Some("abd"));
                assert_eq!(vm.is_match("azd"), Some("azd"));
                assert_eq!(vm.is_match("axd"), Some("axd"));
                assert_eq!(vm.is_match("ad"), None);
                assert_eq!(vm.is_match("aad"), None);
                assert_eq!(vm.is_match("zabd"), Some("abd"));
                assert_eq!(vm.is_match("abdz"), Some("abd"));
            }
            {
                let src = "[b-z]";
                let vm = run(src);

                assert_eq!(vm.is_match("b"), Some("b"));
                assert_eq!(vm.is_match("z"), Some("z"));
                assert_eq!(vm.is_match("x"), Some("x"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("ab"), Some("b"));
                assert_eq!(vm.is_match("bz"), Some("b"));
            }
            {
                let src = "[bcd]";
                let vm = run(src);

                assert_eq!(vm.is_match("b"), Some("b"));
                assert_eq!(vm.is_match("c"), Some("c"));
                assert_eq!(vm.is_match("d"), Some("d"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("e"), None);
                assert_eq!(vm.is_match("ab"), Some("b"));
                assert_eq!(vm.is_match("bz"), Some("b"));
            }
            {
                let src = "a[bc-yz]d";
                let vm = run(src);

                assert_eq!(vm.is_match("abd"), Some("abd"));
                assert_eq!(vm.is_match("azd"), Some("azd"));
                assert_eq!(vm.is_match("acd"), Some("acd"));
                assert_eq!(vm.is_match("ayd"), Some("ayd"));
                assert_eq!(vm.is_match("axd"), Some("axd"));
                assert_eq!(vm.is_match("aad"), None);
                assert_eq!(vm.is_match("ad"), None);
                assert_eq!(vm.is_match("zabd"), Some("abd"));
                assert_eq!(vm.is_match("abdz"), Some("abd"));
            }
            {
                let src = "[z-z]";
                let vm = run(src);

                assert_eq!(vm.is_match("z"), Some("z"));
                assert_eq!(vm.is_match("a"), None);
                assert_eq!(vm.is_match("az"), Some("z"));
                assert_eq!(vm.is_match("za"), Some("z"));
            }
        }

        #[test]
        fn negative() {
            {
                let src = "a[^b-z]d";
                let vm = run(src);

                assert_eq!(vm.is_match("abd"), None);
                assert_eq!(vm.is_match("azd"), None);
                assert_eq!(vm.is_match("axd"), None);
                assert_eq!(vm.is_match("aad"), Some("aad"));
                assert_eq!(vm.is_match("ad"), None);
                assert_eq!(vm.is_match("zaad"), Some("aad"));
                assert_eq!(vm.is_match("aadz"), Some("aad"));
            }
            {
                let src = "[^b-z]";
                let vm = run(src);

                assert_eq!(vm.is_match("b"), None);
                assert_eq!(vm.is_match("z"), None);
                assert_eq!(vm.is_match("x"), None);
                assert_eq!(vm.is_match("a"), Some("a"));
                assert_eq!(vm.is_match("za"), Some("a"));
                assert_eq!(vm.is_match("az"), Some("a"));
            }
            {
                let src = "[^bcd]";
                let vm = run(src);

                assert_eq!(vm.is_match("b"), None);
                assert_eq!(vm.is_match("c"), None);
                assert_eq!(vm.is_match("d"), None);
                assert_eq!(vm.is_match("a"), Some("a"));
                assert_eq!(vm.is_match("e"), Some("e"));
                assert_eq!(vm.is_match("ba"), Some("a"));
                assert_eq!(vm.is_match("ab"), Some("a"));
            }
            {
                let src = "a[^bc-yz]d";
                let vm = run(src);

                assert_eq!(vm.is_match("abd"), None);
                assert_eq!(vm.is_match("azd"), None);
                assert_eq!(vm.is_match("acd"), None);
                assert_eq!(vm.is_match("ayd"), None);
                assert_eq!(vm.is_match("axd"), None);
                assert_eq!(vm.is_match("aad"), Some("aad"));
                assert_eq!(vm.is_match("ad"), None);
                assert_eq!(vm.is_match("zaad"), Some("aad"));
                assert_eq!(vm.is_match("aadz"), Some("aad"));
            }
            {
                let src = "[^z-z]";
                let vm = run(src);

                assert_eq!(vm.is_match("z"), None);
                assert_eq!(vm.is_match("a"), Some("a"));
                assert_eq!(vm.is_match("za"), Some("a"));
                assert_eq!(vm.is_match("az"), Some("a"));
            }
        }
    }

    #[test]
    fn pattern001() {
        {
            let src = r"[a-zA-Z0-9_\.\+\-]+@[a-zA-Z0-9_\.]+[a-zA-Z]+";
            let vm = run(src);

            assert_eq!(vm.is_match("abc@example.com"), Some("abc@example.com"));
            assert_eq!(
                vm.is_match("abc+123@me.example.com"),
                Some("abc+123@me.example.com")
            );
            assert_eq!(vm.is_match("abc@example"), Some("abc@example"));
            assert_eq!(vm.is_match("abc@example.123"), Some("abc@example"));
            assert_eq!(vm.is_match("abc@def@example.com"), Some("abc@def"));
        }
        {
            let src = r"^[a-zA-Z0-9_\.\+\-]+@[a-zA-Z0-9_\.]+[a-zA-Z]+$";
            let vm = run(src);

            assert_eq!(vm.is_match("abc@example.com"), Some("abc@example.com"));
            assert_eq!(
                vm.is_match("abc+123@me.example.com"),
                Some("abc+123@me.example.com")
            );
            assert_eq!(vm.is_match("abc@example"), Some("abc@example"));
            assert_eq!(vm.is_match("abc@example.123"), None);
            assert_eq!(vm.is_match("abc@def@example.com"), None);
        }
    }
}
