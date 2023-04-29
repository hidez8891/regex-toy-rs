use self::{compile::Compiler, exec::Executer, inst::Inst};
use crate::parser::Parser;

mod compile;
mod exec;
mod inst;

#[cfg(test)]
mod tests;

pub struct Vm {
    insts: Vec<Inst>,
    capture_size: usize,
}

impl Vm {
    pub fn new(pattern: &str) -> Result<Vm, String> {
        let ast = Parser::parse(pattern)?;
        let (insts, capture_size) = Compiler::compile(&ast);

        Ok(Vm {
            insts,
            capture_size,
        })
    }

    pub fn is_match<'a>(&self, str: &'a str) -> bool {
        let mut exec = Executer::new(&self.insts, self.capture_size);
        exec.capture_mode(false);
        !exec.execute(str).is_empty()
    }

    pub fn captures<'a>(&self, str: &'a str) -> Vec<&'a str> {
        let mut exec = Executer::new(&self.insts, self.capture_size);
        exec.capture_mode(true);
        exec.execute(str)
    }

    #[cfg(test)]
    pub fn dump(&self) {
        for inst in self.insts.iter() {
            println!("{:?}", inst);
        }
    }
}
