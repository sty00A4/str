use std::{fmt::{Display, Debug}, collections::HashMap, hash::Hash};

use crate::{lexer::{Instr, Position, Token}, error::{Error}};
use crate::error;
use crate::error_pos;
use crate::value::{Type, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Stack {
    stack: Vec<Value>
}
impl Stack {
    pub fn new() -> Self { Self { stack: vec![] } }
    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }
    pub fn pop(&mut self) -> Option<Value> {
        self.stack.pop()
    }
    pub fn peek(&self) -> Option<&Value> {
        self.stack.last()
    }
    pub fn len(&self) -> usize { self.stack.len() }
}
impl Display for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.stack.iter().map(|v| format!("{v:?}")).collect::<Vec<String>>().join(" "))
    }
}

pub enum MacroType {
    Macro(Vec<Token>), Operation(fn(&mut Program) -> Result<(), Error>)
}

pub struct MacroOverload {
    macros: HashMap<Vec<Type>, MacroType>
}
impl MacroOverload {
    pub fn new() -> Self { Self { macros: HashMap::new() } }
    pub fn from(args: Vec<Type>, macro_type: MacroType) -> Self {
        let mut macros = HashMap::new();
        macros.insert(args, macro_type);
        Self { macros }
    }
    pub fn get(&self, stack: &Stack) -> Option<&MacroType> {
        'macros: for (types, macro_type) in self.macros.iter() {
            if stack.len() >= types.len() {
                for (idx, typ) in types.iter().rev().enumerate() {
                    if &stack.stack[stack.len() - 1 - idx].typ() != typ {
                        continue 'macros;
                    }
                }
                return Some(macro_type)
            }
        }
        None
    }
    pub fn def(&mut self, args: Vec<Type>, macro_type: MacroType) -> Option<MacroType> {
        self.macros.insert(args, macro_type)
    }
    pub fn display(&self, id: &String) -> String {
        let mut string = String::new();
        for (types, macro_type) in self.macros.iter() {
            string.push('[');
            string.push_str(types.iter().map(|typ| typ.to_string()).collect::<Vec<String>>().join(" ").as_str());
            string.push_str("] ");
            string.push_str(id.as_str());
        }
        string
    }
}

pub struct Program {
    pub vars: HashMap<String, Value>,
    pub macros: HashMap<String, MacroOverload>,
    pub stack: Stack
}
impl Program {
    pub fn new() -> Self { Self { vars: HashMap::new(), macros: HashMap::new(), stack: Stack::new() } }
    pub fn display_macro(&self, id: &String) -> String {
        if let Some(macro_overload) = self.macros.get(id) {
            macro_overload.display(id)
        } else {
            String::from("no definition found")
        }
    }
    pub fn run(&mut self, tokens: Vec<Token>) -> Result<(), Error> {
        let mut idx = 0;
        for token in tokens {
            match token.instr {
                Instr::String(string) => self.stack.push(Value::String(string)),
                Instr::Char(char) => self.stack.push(Value::Char(char)),
                Instr::Int(int) => self.stack.push(Value::Int(int)),
                Instr::Float(float) => self.stack.push(Value::Float(float)),
                Instr::Boolean(boolean) => self.stack.push(Value::Boolean(boolean)),
                Instr::Take(ids) => {
                    for id in ids {
                        if let Some(value) = self.stack.pop() {
                            self.vars.insert(id, value);
                        } else {
                            return error_pos!(&token.pos, "cannot take value to {id:?} due to stack underflow")
                        }
                    }
                }
                Instr::CopyTo(ids) => {
                    for id in ids {
                        if let Some(value) = self.stack.peek() {
                            self.vars.insert(id, value.clone());
                        } else {
                            return error_pos!(&token.pos, "cannot take value to {id:?} due to stack underflow")
                        }
                    }
                }
                Instr::Copy(token) => match &token.instr {
                    Instr::ID(id) => match self.vars.get(id) {
                        Some(value) => self.stack.push(value.clone()),
                        None => return error_pos!(&token.pos, "unknown id {id:?}")
                    }
                    Instr::CopyTo(ids) => {
                        for id in ids.iter().rev() {
                            match self.vars.get(id) {
                                Some(value) => self.stack.push(value.clone()),
                                None => return error_pos!(&token.pos, "unknown id {id:?}")
                            }
                        }
                    }
                    _ => return error_pos!(&token.pos, "expected identifier or copy-to-indentifiers, got {}", token.instr.name())
                }
                Instr::ID(id) => match self.macros.get(&id) {
                    Some(macros) => match macros.get(&self.stack) {
                        Some(macro_type) => match macro_type {
                            MacroType::Macro(tokens) => self.run(tokens.clone())?,
                            MacroType::Operation(func) => func(self)?,
                        }
                        None => return error_pos!(&token.pos,
                            "no macro definition {id:?} found with current stack, following macros are defined:\n{}\n", self.display_macro(&id))
                    }
                    None => match self.vars.remove(&id) {
                        Some(value) => self.stack.push(value),
                        None => return error_pos!(&token.pos, "unknown id {id:?}")
                    }
                }
            }
            idx += 1;
        }
        Ok(())
    }
    pub fn std_program() -> Self {
        let mut macros = HashMap::new();
        // drop
        let mut drop = MacroOverload::new();
        drop.def(vec![Type::Any], MacroType::Operation(_drop));
        macros.insert(String::from("drop"), drop);
        // copy
        let mut copy = MacroOverload::new();
        copy.def(vec![Type::Any], MacroType::Operation(_copy));
        macros.insert(String::from("copy"), copy);
        // swap
        let mut swap = MacroOverload::new();
        swap.def(vec![Type::Any, Type::Any], MacroType::Operation(_swap));
        macros.insert(String::from("swap"), swap);

        Self { vars: HashMap::new(), macros, stack: Stack::new() }
    }
}

fn _drop(program: &mut Program) -> Result<(), Error> {
    program.stack.pop();
    Ok(())
}
fn _copy(program: &mut Program) -> Result<(), Error> {
    let a = program.stack.peek().unwrap();
    program.stack.push(a.clone());
    Ok(())
}
fn _swap(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    program.stack.push(b);
    program.stack.push(a);
    Ok(())
}