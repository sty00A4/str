use std::fmt::Display;

// use std::ops::Range;
use crate::error;

// #[derive(Debug, Clone, PartialEq)]
// pub struct Position {
//     idx: Range<usize>,
//     ln: Range<usize>,
//     col: Range<usize>,
// }
// impl Position {
//     pub fn new(idx: Range<usize>, ln: Range<usize>, col: Range<usize>) -> Self {
//         Self { idx, ln, col }
//     }
//     pub fn extend(&mut self, pos: Position) {
//         self.idx.end = pos.idx.end;
//         self.ln.end = pos.ln.end;
//         self.col.end = pos.col.end;
//     }
// }

pub const SYMBOLS: [char; 7] = ['"', '\'', '(', ')', '{', '}', '@'];

#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    String(String), Char(char), Int(i64), Float(f64), Boolean(bool),
    ID(String), Take(Vec<String>), CopyTo(Vec<String>), Copy(Box<Instr>)
}
impl Instr {
    pub fn get(id: String) -> Result<Self, String> {
        match id.as_str() {
            "true" => Ok(Self::Boolean(true)),
            "false" => Ok(Self::Boolean(false)),
            _ => match id.chars().next() {
                Some(c) if c.is_digit(10) => match id.parse::<i64>() {
                    Ok(number) => Ok(Self::Int(number)),
                    Err(_) => match id.parse::<f64>() {
                        Ok(number) => Ok(Self::Float(number)),
                        Err(e) => error!("error occurd while parsing the number {id:?}: {e}")
                    }
                }
                Some(_) => Ok(Self::ID(id)),
                None => error!("empty id")
            }
        }
    }
    pub fn name(&self) -> String {
        match self {
            Self::String(_) => format!("string"),
            Self::Char(_) => format!("char"),
            Self::Int(_) => format!("int"),
            Self::Float(_) => format!("float"),
            Self::Boolean(_) => format!("boolean"),
            Self::ID(_) => format!("identifier"),
            Self::Take(_) => format!("take-into-identifiers"),
            Self::CopyTo(_) => format!("copt-to-identifiers"),
            Self::Copy(instr) => format!("copy of {}", instr.name()),
        }
    }
}
impl Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(string) => write!(f, "{string:?}"),
            Self::Char(char) => write!(f, "{char:?}"),
            Self::Int(int) => write!(f, "{int:?}"),
            Self::Float(float) => write!(f, "{float:?}"),
            Self::Boolean(boolean) => write!(f, "{boolean:?}"),
            Self::ID(id) => write!(f, "{id}"),
            Self::Take(ids) => write!(f, "({})", ids.iter().map(|id| id.to_string()).collect::<Vec<String>>().join(" ")),
            Self::CopyTo(ids) => write!(f, "{{{}}}", ids.iter().map(|id| id.to_string()).collect::<Vec<String>>().join(" ")),
            Self::Copy(instr) => write!(f, "@{instr}"),
        }
    }
}

pub struct Lexer {
    text: String,
    idx: usize,
    ln: usize,
    col: usize
}
impl Lexer {
    pub fn new(text: String) -> Self { Self { text, idx: 0, ln: 0, col: 0 } }
    pub fn get(&self) -> Option<char> {
        self.text.get(self.idx..self.idx+1)?.chars().next()
    }
    pub fn advance(&mut self) {
        self.idx += 1;
        self.col += 1;
        if self.get() == Some('\n') {
            self.ln += 1;
            self.col = 0;
        }
    }
    pub fn advance_ws(&mut self) {
        while let Some(c) = self.get() {
            if !c.is_whitespace() || SYMBOLS.contains(&c) { break }
            self.advance();
        }
    }
    pub fn next(&mut self) -> Result<Option<Instr>, String> {
        self.advance_ws();
        match self.get() {
            Some('"') => {
                self.advance();
                let mut string = String::new();
                while let Some(c) = self.get() {
                    if c == '"' { break }
                    string.push(c);
                    self.advance();
                }
                if self.get() == None { return error!("unclosed string") }
                self.advance();
                Ok(Some(Instr::String(string)))
            }
            Some('\'') => {
                self.advance();
                if let Some(char) = self.get() {
                    self.advance();
                    if self.get() != Some('\'') { return error!("unclosed character") }
                    self.advance();
                    Ok(Some(Instr::Char(char)))
                } else {
                    error!("expected character")
                }
            }
            Some('(') => {
                self.advance();
                let mut ids: Vec<String> = vec![];
                while let Some(c) = self.get() {
                    if c == ')' { break }
                    if let Some(instr) = self.next()? {
                        match instr {
                            Instr::ID(id) => ids.push(id),
                            _ => return error!("expected identifier, got {}", instr.name())
                        }
                    } else {
                        return error!("unclosed identifier take")
                    }
                }
                if self.get() == None { return error!("unclosed identifier take") }
                self.advance();
                Ok(Some(Instr::Take(ids.iter().rev().map(|id| id.clone()).collect())))
            }
            Some('{') => {
                self.advance();
                let mut ids: Vec<String> = vec![];
                while let Some(c) = self.get() {
                    if c == '}' { break }
                    if let Some(instr) = self.next()? {
                        match instr {
                            Instr::ID(id) => ids.push(id),
                            _ => return error!("expected identifier, got {}", instr.name())
                        }
                    } else {
                        return error!("unclosed identifier copy")
                    }
                }
                if self.get() == None { return error!("unclosed identifier copy") }
                self.advance();
                Ok(Some(Instr::CopyTo(ids.iter().rev().map(|id| id.clone()).collect())))
            }
            Some('@') => {
                self.advance();
                if let Some(instr) = self.next()? {
                    Ok(Some(Instr::Copy(Box::new(instr))))
                } else {
                    return error!("unexpected end")
                }
            }
            Some(c) => {
                self.advance();
                let mut id = String::from(c);
                while let Some(c) = self.get() {
                    if c.is_whitespace() || SYMBOLS.contains(&c) { break }
                    id.push(c);
                    self.advance();
                }
                Ok(Some(Instr::get(id)?))
            }
            None => Ok(None)
        }
    }
    pub fn lex(&mut self) -> Result<Vec<Instr>, String> {
        let mut instrs = vec![];
        while let Some(instr) = self.next()? {
            instrs.push(instr);
        }
        Ok(instrs)
    }
}

pub fn lex(text: String) -> Result<Vec<Instr>, String> {
    Lexer::new(text).lex()
}