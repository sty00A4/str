use std::fmt::{Display, Debug};

use std::ops::Range;
use crate::error;
use crate::error::{Error};
use crate::error_pos;

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub idx: Range<usize>,
    pub ln: Range<usize>,
    pub col: Range<usize>,
}
impl Position {
    pub fn new(idx: Range<usize>, ln: Range<usize>, col: Range<usize>) -> Self {
        Self { idx, ln, col }
    }
    pub fn zero() -> Self {
        Self { idx: 0..1, ln: 0..1, col: 0..1 }
    }
    pub fn extend(&mut self, pos: Position) {
        self.idx.end = pos.idx.end;
        self.ln.end = pos.ln.end;
        self.col.end = pos.col.end;
    }
}
impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.ln.start + 1, self.col.start + 1)
    }
}

pub const SYMBOLS: [char; 7] = ['"', '\'', '(', ')', '{', '}', '@'];

#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    String(String), Char(char), Int(i64), Float(f64), Boolean(bool),
    ID(String), Take(Vec<String>), CopyTo(Vec<String>), Copy(Box<Token>),
    End, If, Else, Repeat, Macro
}
impl Instr {
    pub fn get(id: String, pos: Position) -> Result<Self, Error> {
        match id.as_str() {
            "true" => Ok(Self::Boolean(true)),
            "false" => Ok(Self::Boolean(false)),
            "end" => Ok(Self::End),
            "if" => Ok(Self::If),
            "else" => Ok(Self::Else),
            "repeat" => Ok(Self::Repeat),
            "macro" => Ok(Self::Macro),
            _ => match id.chars().next() {
                Some(c) if c.is_digit(10) => match id.parse::<i64>() {
                    Ok(number) => Ok(Self::Int(number)),
                    Err(_) => match id.parse::<f64>() {
                        Ok(number) => Ok(Self::Float(number)),
                        Err(e) => error_pos!(pos, "error occurd while parsing the number {id:?}: {e}")
                    }
                }
                Some(_) => Ok(Self::ID(id)),
                None => error_pos!(pos, "empty id")
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
            Self::Copy(token) => format!("copy of {}", token.instr.name()),
            Self::End => format!("end-control-flow instruction"),
            Self::If => format!("if-control-flow instruction"),
            Self::Else => format!("else-control-flow instruction"),
            Self::Repeat => format!("repeat-control-flow instruction"),
            Self::Macro => format!("macro instruction"),
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
            Self::End => write!(f, ";"),
            Self::If => write!(f, "if"),
            Self::Else => write!(f, "else"),
            Self::Repeat => write!(f, "repeat"),
            Self::Macro => write!(f, "macro"),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Token {
    pub instr: Instr,
    pub pos: Position
}
impl Token {
    pub fn new(instr: Instr, pos: Position) -> Self { Self { instr, pos } }
}
impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.instr)
    }
}
impl Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.instr)
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
    pub fn pos(&self) -> Position {
        Position::new(self.idx..self.idx+1, self.ln..self.ln+1, self.col..self.col+1)
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
    pub fn next(&mut self) -> Result<Option<Token>, Error> {
        self.advance_ws();
        let mut pos = self.pos();
        match self.get() {
            Some('"') => {
                self.advance();
                let mut string = String::new();
                while let Some(c) = self.get() {
                    if c == '"' { break }
                    string.push(c);
                    self.advance();
                }
                if self.get() == None { return error_pos!(pos, "unclosed string") }
                pos.extend(self.pos());
                self.advance();
                Ok(Some(Token::new(Instr::String(string), pos)))
            }
            Some('\'') => {
                self.advance();
                if let Some(char) = self.get() {
                    self.advance();
                    if self.get() != Some('\'') { return error_pos!(pos, "unclosed character") }
                    pos.extend(self.pos());
                    self.advance();
                    Ok(Some(Token::new(Instr::Char(char), pos)))
                } else {
                    error_pos!(pos, "expected character")
                }
            }
            Some('(') => {
                self.advance();
                let mut ids: Vec<String> = vec![];
                while let Some(c) = self.get() {
                    if c == ')' { break }
                    if let Some(token) = self.next()? {
                        match token.instr {
                            Instr::ID(id) => ids.push(id),
                            _ => return error_pos!(pos, "expected identifier, got {}", token.instr.name())
                        }
                    } else {
                        return error_pos!(pos, "unclosed identifier take")
                    }
                }
                if self.get() == None { return error_pos!(pos, "unclosed identifier take") }
                pos.extend(self.pos());
                self.advance();
                Ok(Some(Token::new(Instr::Take(ids.iter().rev().map(|id| id.clone()).collect()), pos)))
            }
            Some('{') => {
                self.advance();
                let mut ids: Vec<String> = vec![];
                while let Some(c) = self.get() {
                    if c == '}' { break }
                    if let Some(token) = self.next()? {
                        match token.instr {
                            Instr::ID(id) => ids.push(id),
                            _ => return error_pos!(pos, "expected identifier, got {}", token.instr.name())
                        }
                    } else {
                        return error_pos!(pos, "unclosed identifier copy")
                    }
                }
                if self.get() == None { return error_pos!(pos, "unclosed identifier copy") }
                pos.extend(self.pos());
                self.advance();
                Ok(Some(Token::new(Instr::CopyTo(ids.iter().rev().map(|id| id.clone()).collect()), pos)))
            }
            Some('@') => {
                self.advance();
                if let Some(token) = self.next()? {
                    pos.extend(token.pos.clone());
                    Ok(Some(Token::new(Instr::Copy(Box::new(token)), pos)))
                } else {
                    return error_pos!(pos, "unexpected end")
                }
            }
            Some('#') => {
                while let Some(c) = self.get() {
                    if c == '\n' { self.advance(); break }
                    self.advance();
                }
                self.next()
            }
            Some(c) => {
                self.advance();
                let mut id = String::from(c);
                while let Some(c) = self.get() {
                    if c.is_whitespace() || SYMBOLS.contains(&c) { break }
                    id.push(c);
                    pos.extend(self.pos());
                    self.advance();
                }
                Ok(Some(Token::new(Instr::get(id, pos.clone())?, pos)))
            }
            None => Ok(None)
        }
    }
    pub fn lex(&mut self) -> Result<Vec<Token>, Error> {
        let mut tokens = vec![];
        while let Some(token) = self.next()? {
            tokens.push(token);
        }
        Ok(tokens)
    }
}

pub fn lex(text: String) -> Result<Vec<Token>, Error> {
    Lexer::new(text).lex()
}