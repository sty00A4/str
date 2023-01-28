use std::{fmt::{Display, Debug}, collections::HashMap};

use crate::{lexer::{Instr, Position, Token}, error::{Error}};
use crate::error;
use crate::error_pos;

#[derive(Clone, PartialEq)]
pub enum Value {
    String(String), Char(char), Int(i64), Float(f64), Boolean(bool)
}
impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(string) => write!(f, "{string:?}"),
            Self::Char(char) => write!(f, "{char:?}"),
            Self::Int(int) => write!(f, "{int:?}"),
            Self::Float(float) => write!(f, "{float:?}"),
            Self::Boolean(boolean) => write!(f, "{boolean:?}"),
        }
    }
}
impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(string) => write!(f, "{string}"),
            Self::Char(char) => write!(f, "{char}"),
            Self::Int(int) => write!(f, "{int}"),
            Self::Float(float) => write!(f, "{float}"),
            Self::Boolean(boolean) => write!(f, "{boolean}"),
        }
    }
}

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
}
impl Display for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.stack.iter().map(|v| format!("{v:?}")).collect::<Vec<String>>().join(" "))
    }
}

pub struct Program {
    pub vars: HashMap<String, Value>,
    pub stack: Stack
}
impl Program {
    pub fn new() -> Self { Self { vars: HashMap::new(), stack: Stack::new() } }
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
                Instr::ID(id) => match id.as_str() {
                    "print" => {
                        if let Some(value) = self.stack.pop() {
                            println!("{value}");
                        }
                    }
                    "drop" => {
                        self.stack.pop();
                    }
                    "copy" => {
                        if let Some(a) = self.stack.peek() {
                            self.stack.push(a.clone());
                        } else {
                            return error_pos!(&token.pos, "cannot perform {id:?} due to stack underflow")
                        }
                    }
                    "swap" => {
                        if let (Some(a), Some(b)) = (self.stack.pop(), self.stack.pop()) {
                            self.stack.push(a);
                            self.stack.push(b);
                        } else {
                            return error_pos!(&token.pos, "cannot perform {id:?} due to stack underflow")
                        }
                    }
                    _ => match self.vars.remove(&id) {
                        Some(value) => self.stack.push(value),
                        None => return error_pos!(&token.pos, "unknown id {id:?}")
                    }
                }
            }
            idx += 1;
        }
        Ok(())
    }
}