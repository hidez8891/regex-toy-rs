use super::{compile::Compiler, exec::Executer, inst::Inst};
use crate::parser::Parser;

pub struct Vm {
    insts: Vec<Inst>,
}

impl Vm {
    pub fn new(pattern: &str) -> Result<Vm, String> {
        let ast = Parser::parse(pattern)?;
        let insts = Compiler::compile(&ast);
        Ok(Vm { insts })
    }

    pub fn is_match<'a>(&self, str: &'a str) -> Option<&'a str> {
        let mut exec = Executer::new(&self.insts);
        exec.execute(str)
    }

    #[cfg(test)]
    pub fn dump(&self) {
        for inst in self.insts.iter() {
            println!("{:?}", inst);
        }
    }
}
