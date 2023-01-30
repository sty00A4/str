use std::{fmt::{Display, Debug}, collections::HashMap, hash::Hash};

use crate::{lexer::{Instr, Position, Token}, error::{Error}, parser::{Node, NodeType}};
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
    Macro(Node), Operation(fn(&mut Program) -> Result<(), Error>)
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
            string.push('\n');
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
    pub fn run(&mut self, node: Node) -> Result<(), Error> {
        let mut idx = 0;
        match node.node {
            NodeType::Chunk(nodes) => {
                for node in nodes {
                    self.run(node)?;
                }
            }
            NodeType::String(string) => self.stack.push(Value::String(string)),
            NodeType::Char(char) => self.stack.push(Value::Char(char)),
            NodeType::Int(int) => self.stack.push(Value::Int(int)),
            NodeType::Float(float) => self.stack.push(Value::Float(float)),
            NodeType::Boolean(boolean) => self.stack.push(Value::Boolean(boolean)),
            NodeType::Take(ids) => {
                for id in ids {
                    if let Some(value) = self.stack.pop() {
                        self.vars.insert(id, value);
                    } else {
                        return error_pos!(&node.pos, "cannot take value to {id:?} due to stack underflow")
                    }
                }
            }
            NodeType::CopyTo(ids) => {
                for id in ids {
                    if let Some(value) = self.stack.peek() {
                        self.vars.insert(id, value.clone());
                    } else {
                        return error_pos!(&node.pos, "cannot take value to {id:?} due to stack underflow")
                    }
                }
            }
            NodeType::Copy(token) => match &token.instr {
                Instr::ID(id) => match self.vars.get(id) {
                    Some(value) => self.stack.push(value.clone()),
                    None => match self.macros.get(id) {
                        Some(_) => return error_pos!(&token.pos, "cannot copy a macro, {id:?} is defined as a macro"),
                        None => return error_pos!(&token.pos, "unknown id {id:?}")
                    }
                }
                Instr::CopyTo(ids) => {
                    for id in ids.iter().rev() {
                        match self.vars.get(id) {
                            Some(value) => self.stack.push(value.clone()),
                            None => match self.macros.get(id) {
                                Some(_) => return error_pos!(&token.pos, "cannot copy a macro, {id:?} is defined as a macro"),
                                None => return error_pos!(&token.pos, "unknown id {id:?}")
                            }
                        }
                    }
                }
                _ => return error_pos!(&token.pos, "expected identifier or copy-to-indentifiers, got {}", token.instr.name())
            }
            NodeType::ID(id) => match self.macros.get(&id) {
                Some(macros) => match macros.get(&self.stack) {
                    Some(macro_type) => match macro_type {
                        MacroType::Macro(node) => self.run(node.clone())?,
                        MacroType::Operation(func) => func(self)?,
                    }
                    None => return error_pos!(&node.pos,
                        "no macro definition {id:?} found with current stack, following macros are defined:\n{}\n", self.display_macro(&id))
                }
                None => match self.vars.remove(&id) {
                    Some(value) => self.stack.push(value),
                    None => return error_pos!(&node.pos, "unknown id {id:?}")
                }
            }
            NodeType::If(case_node, else_node) => {
                let Some(cond) = self.stack.pop() else {
                    return error_pos!(&node.pos, "couldn't perform if-control-flow operation due to stack underflow");
                };
                if let Value::Boolean(cond) = cond {
                    if cond {
                        self.run(*case_node);
                    } else if let Some(else_node) = else_node {
                        self.run(*else_node);
                    }
                } else {
                    return error_pos!(&node.pos, "expected a boolean value on top of the stack, got {}", cond.typ())
                }
            }
            NodeType::Repeat(body) => {
                let Some(count) = self.stack.pop() else {
                    return error_pos!(&node.pos, "couldn't perform if-control-flow operation due to stack underflow");
                };
                if let Value::Int(count) = count {
                    for _ in 0..count {
                        self.run(*body.clone());
                    }
                } else {
                    return error_pos!(&node.pos, "expected a boolean value on top of the stack, got {}", count.typ())
                }
            }
            NodeType::Macro(name, types, body) => todo!("macro definition"),
        }
        Ok(())
    }
    pub fn std_program() -> Self {
        let mut macros = HashMap::new();
        // LEN
        let mut stack_len = MacroOverload::new();
        stack_len.def(vec![], MacroType::Operation(_stack_len));
        macros.insert(String::from("LEN"), stack_len);
        // len
        let mut len = MacroOverload::new();
        len.def(vec![Type::String], MacroType::Operation(_len));
        macros.insert(String::from("len"), len);
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
        // over
        let mut over = MacroOverload::new();
        over.def(vec![Type::Any, Type::Any], MacroType::Operation(_over));
        macros.insert(String::from("over"), over);
        // +
        let mut add = MacroOverload::new();
        add.def(vec![Type::Int, Type::Int], MacroType::Operation(_add));
        add.def(vec![Type::Float, Type::Float], MacroType::Operation(_add));
        add.def(vec![Type::Int, Type::Float], MacroType::Operation(_add));
        add.def(vec![Type::Float, Type::Int], MacroType::Operation(_add));
        add.def(vec![Type::String, Type::String], MacroType::Operation(_add));
        add.def(vec![Type::String, Type::Char], MacroType::Operation(_add));
        macros.insert(String::from("+"), add);
        // -
        let mut sub = MacroOverload::new();
        sub.def(vec![Type::Int, Type::Int], MacroType::Operation(_sub));
        sub.def(vec![Type::Float, Type::Float], MacroType::Operation(_sub));
        sub.def(vec![Type::Int, Type::Float], MacroType::Operation(_sub));
        sub.def(vec![Type::Float, Type::Int], MacroType::Operation(_sub));
        macros.insert(String::from("-"), sub);
        // *
        let mut mult = MacroOverload::new();
        mult.def(vec![Type::Int, Type::Int], MacroType::Operation(_mult));
        mult.def(vec![Type::Float, Type::Float], MacroType::Operation(_mult));
        mult.def(vec![Type::Int, Type::Float], MacroType::Operation(_mult));
        mult.def(vec![Type::Float, Type::Int], MacroType::Operation(_mult));
        mult.def(vec![Type::String, Type::Int], MacroType::Operation(_mult));
        mult.def(vec![Type::Char, Type::Int], MacroType::Operation(_mult));
        macros.insert(String::from("*"), mult);
        // /
        let mut div = MacroOverload::new();
        div.def(vec![Type::Int, Type::Int], MacroType::Operation(_div));
        div.def(vec![Type::Float, Type::Float], MacroType::Operation(_div));
        div.def(vec![Type::Int, Type::Float], MacroType::Operation(_div));
        div.def(vec![Type::Float, Type::Int], MacroType::Operation(_div));
        macros.insert(String::from("/"), div);
        // %
        let mut module = MacroOverload::new();
        module.def(vec![Type::Int, Type::Int], MacroType::Operation(_module));
        module.def(vec![Type::Float, Type::Float], MacroType::Operation(_module));
        module.def(vec![Type::Int, Type::Float], MacroType::Operation(_module));
        module.def(vec![Type::Float, Type::Int], MacroType::Operation(_module));
        macros.insert(String::from("%"), module);
        // and
        let mut and = MacroOverload::new();
        and.def(vec![Type::Boolean, Type::Boolean], MacroType::Operation(_and));
        macros.insert(String::from("and"), and);
        // or
        let mut or = MacroOverload::new();
        or.def(vec![Type::Boolean, Type::Boolean], MacroType::Operation(_or));
        macros.insert(String::from("or"), or);
        // not
        let mut not = MacroOverload::new();
        not.def(vec![Type::Boolean], MacroType::Operation(_not));
        macros.insert(String::from("not"), not);
        // =
        let mut eq = MacroOverload::new();
        eq.def(vec![Type::Any, Type::Any], MacroType::Operation(_eq));
        macros.insert(String::from("="), eq);
        // !=
        let mut ne = MacroOverload::new();
        ne.def(vec![Type::Any, Type::Any], MacroType::Operation(_ne));
        macros.insert(String::from("!="), ne);
        // >
        let mut lt = MacroOverload::new();
        lt.def(vec![Type::Int, Type::Int], MacroType::Operation(_lt));
        lt.def(vec![Type::Float, Type::Float], MacroType::Operation(_lt));
        lt.def(vec![Type::Int, Type::Float], MacroType::Operation(_lt));
        lt.def(vec![Type::Float, Type::Int], MacroType::Operation(_lt));
        macros.insert(String::from("<"), lt);
        // <
        let mut gt = MacroOverload::new();
        gt.def(vec![Type::Int, Type::Int], MacroType::Operation(_gt));
        gt.def(vec![Type::Float, Type::Float], MacroType::Operation(_gt));
        gt.def(vec![Type::Int, Type::Float], MacroType::Operation(_gt));
        gt.def(vec![Type::Float, Type::Int], MacroType::Operation(_gt));
        macros.insert(String::from(">"), gt);
        // <=
        let mut le = MacroOverload::new();
        le.def(vec![Type::Int, Type::Int], MacroType::Operation(_le));
        le.def(vec![Type::Float, Type::Float], MacroType::Operation(_le));
        le.def(vec![Type::Int, Type::Float], MacroType::Operation(_le));
        le.def(vec![Type::Float, Type::Int], MacroType::Operation(_le));
        macros.insert(String::from("<="), le);
        // >=
        let mut ge = MacroOverload::new();
        ge.def(vec![Type::Int, Type::Int], MacroType::Operation(_ge));
        ge.def(vec![Type::Float, Type::Float], MacroType::Operation(_ge));
        ge.def(vec![Type::Int, Type::Float], MacroType::Operation(_ge));
        ge.def(vec![Type::Float, Type::Int], MacroType::Operation(_ge));
        macros.insert(String::from(">="), ge);

        // .
        let mut index = MacroOverload::new();
        index.def(vec![Type::String, Type::Int], MacroType::Operation(_index));
        index.def(vec![Type::String, Type::Int, Type::Int], MacroType::Operation(_index_range));
        macros.insert(String::from("."), index);
        // rev
        let mut rev = MacroOverload::new();
        rev.def(vec![Type::String], MacroType::Operation(_rev));
        macros.insert(String::from("rev"), rev);
        // pos
        let mut pos = MacroOverload::new();
        pos.def(vec![Type::String, Type::String], MacroType::Operation(_pos));
        pos.def(vec![Type::String, Type::Char], MacroType::Operation(_pos));
        macros.insert(String::from("pos"), pos);
        // remove
        let mut remove = MacroOverload::new();
        remove.def(vec![Type::String, Type::Int], MacroType::Operation(_remove));
        macros.insert(String::from("remove"), remove);
        // count
        let mut count = MacroOverload::new();
        count.def(vec![Type::String, Type::Char], MacroType::Operation(_count));
        count.def(vec![Type::String, Type::String], MacroType::Operation(_count));
        macros.insert(String::from("count"), count);
        // split
        let mut split = MacroOverload::new();
        split.def(vec![Type::String, Type::Char], MacroType::Operation(_split));
        split.def(vec![Type::String, Type::String], MacroType::Operation(_split));
        macros.insert(String::from("split"), split);
        // join
        let mut join = MacroOverload::new();
        join.def(vec![Type::Char], MacroType::Operation(_join));
        join.def(vec![Type::String], MacroType::Operation(_join));
        macros.insert(String::from("join"), join);

        Self { vars: HashMap::new(), macros, stack: Stack::new() }
    }
}

fn _stack_len(program: &mut Program) -> Result<(), Error> {
    program.stack.push(Value::Int(program.stack.len() as i64));
    Ok(())
}
fn _len(program: &mut Program) -> Result<(), Error> {
    let a = program.stack.pop().unwrap();
    match a {
        Value::String(string) => program.stack.push(Value::Int(string.len() as i64)),
        _ => panic!("type checking error!!!")
    }
    Ok(())
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
fn _over(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    let c = a.clone();
    program.stack.push(a);
    program.stack.push(b);
    program.stack.push(c);
    Ok(())
}
fn _add(program: &mut Program) -> Result<(), Error> {
    let (mut b, mut a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a.clone(), b) {
        (Value::Int(v1), Value::Int(v2)) => program.stack.push(Value::Int(v1 + v2)),
        (Value::Float(v1), Value::Float(v2)) => program.stack.push(Value::Float(v1 + v2)),
        (Value::Int(int), Value::Float(float)) |
        (Value::Float(float), Value::Int(int)) => program.stack.push(Value::Float(int as f64 + float)),
        (Value::String(v1), Value::String(v2)) => program.stack.push(Value::String(v1 + &v2)),
        (Value::String(mut v1), Value::Char(v2)) => {
            v1.push(v2);
            program.stack.push(Value::String(v1));
        }
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _sub(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::Int(v1), Value::Int(v2)) => program.stack.push(Value::Int(v1 - v2)),
        (Value::Float(v1), Value::Float(v2)) => program.stack.push(Value::Float(v1 - v2)),
        (Value::Int(int), Value::Float(float)) => program.stack.push(Value::Float(int as f64 - float)),
        (Value::Float(float), Value::Int(int)) => program.stack.push(Value::Float(float - int as f64)),
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _mult(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::Int(v1), Value::Int(v2)) => program.stack.push(Value::Int(v1 * v2)),
        (Value::Float(v1), Value::Float(v2)) => program.stack.push(Value::Float(v1 * v2)),
        (Value::Int(int), Value::Float(float)) => program.stack.push(Value::Float(int as f64 * float)),
        (Value::Float(float), Value::Int(int)) => program.stack.push(Value::Float(float * int as f64)),
        (Value::String(s), Value::Int(rep)) => program.stack.push(Value::String(s.repeat(rep.max(0) as usize))),
        (Value::Char(c), Value::Int(rep)) => program.stack.push(Value::String(c.to_string().repeat(rep.max(0) as usize))),
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _div(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::Int(v1), Value::Int(v2)) => program.stack.push(Value::Float(v1 as f64 / v2 as f64)),
        (Value::Float(v1), Value::Float(v2)) => program.stack.push(Value::Float(v1 / v2)),
        (Value::Int(int), Value::Float(float)) => program.stack.push(Value::Float(int as f64 / float)),
        (Value::Float(float), Value::Int(int)) => program.stack.push(Value::Float(float / int as f64)),
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _module(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::Int(v1), Value::Int(v2)) => program.stack.push(Value::Int(v1 % v2)),
        (Value::Float(v1), Value::Float(v2)) => program.stack.push(Value::Float(v1 % v2)),
        (Value::Int(int), Value::Float(float)) => program.stack.push(Value::Float(int as f64 % float)),
        (Value::Float(float), Value::Int(int)) => program.stack.push(Value::Float(float % int as f64)),
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _pow(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::Int(v1), Value::Int(v2)) => program.stack.push(Value::Int(v1.pow(v2.max(0) as u32))),
        (Value::Float(v1), Value::Float(v2)) => program.stack.push(Value::Float(v1.powf(v2))),
        (Value::Int(int), Value::Float(float)) => program.stack.push(Value::Float((int as f64).powf(float))),
        (Value::Float(float), Value::Int(int)) => program.stack.push(Value::Float((float as f64).powi(int.max(0) as i32))),
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _and(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::Boolean(v1), Value::Boolean(v2)) => program.stack.push(Value::Boolean(v1 && v2)),
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _or(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::Boolean(v1), Value::Boolean(v2)) => program.stack.push(Value::Boolean(v1 || v2)),
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _not(program: &mut Program) -> Result<(), Error> {
    let a = program.stack.pop().unwrap();
    match a {
        Value::Boolean(v) => program.stack.push(Value::Boolean(!v)),
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _eq(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    program.stack.push(Value::Boolean(a == b));
    Ok(())
}
fn _ne(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    program.stack.push(Value::Boolean(a != b));
    Ok(())
}
fn _lt(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::Int(v1), Value::Int(v2)) => program.stack.push(Value::Boolean(v1 < v2)),
        (Value::Float(v1), Value::Float(v2)) => program.stack.push(Value::Boolean(v1 < v2)),
        (Value::Int(int), Value::Float(float)) => program.stack.push(Value::Boolean((int as f64) < float)),
        (Value::Float(float), Value::Int(int)) => program.stack.push(Value::Boolean(float < int as f64)),
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _gt(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::Int(v1), Value::Int(v2)) => program.stack.push(Value::Boolean(v1 > v2)),
        (Value::Float(v1), Value::Float(v2)) => program.stack.push(Value::Boolean(v1 > v2)),
        (Value::Int(int), Value::Float(float)) => program.stack.push(Value::Boolean(int as f64 > float)),
        (Value::Float(float), Value::Int(int)) => program.stack.push(Value::Boolean(float > int as f64)),
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _le(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::Int(v1), Value::Int(v2)) => program.stack.push(Value::Boolean(v1 <= v2)),
        (Value::Float(v1), Value::Float(v2)) => program.stack.push(Value::Boolean(v1 <= v2)),
        (Value::Int(int), Value::Float(float)) => program.stack.push(Value::Boolean(int as f64 <= float)),
        (Value::Float(float), Value::Int(int)) => program.stack.push(Value::Boolean(float <= int as f64)),
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _ge(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::Int(v1), Value::Int(v2)) => program.stack.push(Value::Boolean(v1 >= v2)),
        (Value::Float(v1), Value::Float(v2)) => program.stack.push(Value::Boolean(v1 >= v2)),
        (Value::Int(int), Value::Float(float)) => program.stack.push(Value::Boolean(int as f64 >= float)),
        (Value::Float(float), Value::Int(int)) => program.stack.push(Value::Boolean(float >= int as f64)),
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _index(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    if let (Value::String(string), Value::Int(idx)) = (a, b) {
        let idx = if idx < 0 {
            string.len() - idx.abs() as usize % string.len()
        } else {
            idx.abs() as usize % string.len()
        };
        program.stack.push(Value::Char(string[idx..idx+1].chars().next().unwrap()));
        Ok(())
    } else {
        panic!("type checking error!!!")
    }
}
fn _index_range(program: &mut Program) -> Result<(), Error> {
    let (c, b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap(), program.stack.pop().unwrap());
    if let (Value::String(string), Value::Int(start), Value::Int(end)) = (a, b, c) {
        let start = if start < 0 {
            string.len() - start.abs() as usize % string.len()
        } else {
            start.abs() as usize % string.len()
        };
        let end = if end < 0 {
            string.len() - end.abs() as usize % string.len()
        } else {
            end.abs() as usize % string.len()
        };
        program.stack.push(Value::String(string[start..end].to_string()));
        Ok(())
    } else {
        panic!("type checking error!!!")
    }
}
fn _rev(program: &mut Program) -> Result<(), Error> {
    if let Value::String(string) = program.stack.pop().unwrap() {
        program.stack.push(Value::String(string.chars().rev().collect()));
        Ok(())
    } else {
        panic!("type checking error!!!")
    }
}
fn _pos(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::String(string), Value::Char(char)) => {
            match string.find(char) {
                Some(index) => {
                    program.stack.push(Value::Int(index as i64));
                    program.stack.push(Value::Boolean(true));
                }
                None => program.stack.push(Value::Boolean(false))
            }
        }
        (Value::String(string), Value::String(sub)) => {
            match string.find(&sub) {
                Some(index) => {
                    program.stack.push(Value::Int(index as i64));
                    program.stack.push(Value::Boolean(true));
                }
                None => program.stack.push(Value::Boolean(false))
            }
        }
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _remove(program: &mut Program) -> Result<(), Error> {
    let (b, mut a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::String(mut string), Value::Int(idx)) => {
            let idx = if idx < 0 {
                string.len() - idx.abs() as usize % string.len()
            } else {
                idx.abs() as usize % string.len()
            };
            program.stack.push(Value::Char(string.remove(idx)));
        }
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _count(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::String(string), Value::Char(count_char)) => {
            let mut count: usize = 0;
            for char in string.chars() {
                if char == count_char {
                    count += 1;
                }
            }
            program.stack.push(Value::Int(count as i64));
        }
        (Value::String(string), Value::String(count_string)) => {
            let mut count: usize = 0;
            for idx in 0..string.len() {
                if string.get(idx..idx+count_string.len()) == Some(count_string.as_str()) {
                    count += 1;
                }
            }
            program.stack.push(Value::Int(count as i64));
        }
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _split(program: &mut Program) -> Result<(), Error> {
    let (b, a) = (program.stack.pop().unwrap(), program.stack.pop().unwrap());
    match (a, b) {
        (Value::String(string), Value::Char(pattern)) => {
            let mut parts: Vec<&str> = string.split(pattern).collect();
            let len = parts.len();
            for part in parts {
                program.stack.push(Value::String(part.to_string()));
            }
            program.stack.push(Value::Int(len as i64));
        }
        (Value::String(string), Value::String(pattern)) => {
            let mut parts: Vec<&str> = string.split(pattern.as_str()).collect();
            let len = parts.len();
            for part in parts {
                program.stack.push(Value::String(part.to_string()));
            }
            program.stack.push(Value::Int(len as i64));
        }
        _ => panic!("type checking error!!!")
    }
    Ok(())
}
fn _join(program: &mut Program) -> Result<(), Error> {
    let a = program.stack.pop().unwrap();
    let len = program.stack.len();
    let mut strings = vec![];
    for _ in 0..len {
        let Some(value) = program.stack.pop() else { break };
        if let Value::String(value) = value {
            strings.push(value);
        } else {
            strings.push(value.to_string())
        }
    }
    let strings: Vec<String> = strings.iter().rev().map(|s| s.clone()).collect();
    match a {
        Value::Char(char) => {
            program.stack.push(Value::String(strings.join(char.to_string().as_str())));
        }
        Value::String(string) => {
            program.stack.push(Value::String(strings.join(string.as_str())));
        }
        _ => panic!("type checking error!!!")
    }
    Ok(())
}