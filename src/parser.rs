use crate::error;
use crate::error::Error;
use crate::error_pos;
use crate::value::Type;
use crate::lexer::{Token, Position, Instr};

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Chunk(Vec<Node>),
    String(String), Char(char), Int(i64), Float(f64), Boolean(bool),
    ID(String), Take(Vec<String>), CopyTo(Vec<String>), Copy(Box<Token>),
    If(Box<Node>, Option<Box<Node>>), Repeat(Box<Node>), Macro(String, Vec<Type>, Box<Node>)
}
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub node: NodeType,
    pub pos: Position
}
impl Node {
    pub fn new(node: NodeType, pos: Position) -> Self { Self { node, pos } }
}

pub struct Parser {
    tokens: Vec<Token>,
    idx: usize
}
impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self { Self { tokens, idx: 0 } }
    pub fn get(&self) -> Option<&Token> {
        self.tokens.get(self.idx)
    }
    pub fn pos(&self) -> Option<&Position> {
        Some(&self.tokens.get(self.idx)?.pos)
    }
    pub fn advance(&mut self) {
        self.idx += 1;
    }
    pub fn next(&mut self) -> Result<Option<Node>, Error> {
        match self.get() {
            Some(token) => {
                let mut pos = token.pos.clone();
                match token.instr.clone() {
                    Instr::String(string) => { self.advance(); Ok(Some(Node::new(NodeType::String(string), pos))) }
                    Instr::Char(char) => { self.advance(); Ok(Some(Node::new(NodeType::Char(char), pos))) }
                    Instr::Int(int) => { self.advance(); Ok(Some(Node::new(NodeType::Int(int), pos))) }
                    Instr::Float(float) => { self.advance(); Ok(Some(Node::new(NodeType::Float(float), pos))) }
                    Instr::Boolean(boolean) => { self.advance(); Ok(Some(Node::new(NodeType::Boolean(boolean), pos))) }
                    Instr::ID(id) => { self.advance(); Ok(Some(Node::new(NodeType::ID(id), pos))) }
                    Instr::Take(ids) => { self.advance(); Ok(Some(Node::new(NodeType::Take(ids), pos))) }
                    Instr::Copy(ids) => { self.advance(); Ok(Some(Node::new(NodeType::Copy(ids), pos))) }
                    Instr::CopyTo(instr) => { self.advance(); Ok(Some(Node::new(NodeType::CopyTo(instr), pos))) }
                    Instr::If => {
                        self.advance();
                        let mut nodes = vec![];
                        let mut else_node = None;
                        while let Some(token) = self.get() {
                            if token.instr == Instr::End { self.advance(); break }
                            if token.instr == Instr::Else { break }
                            if let Some(node) = self.next()? {
                                pos.extend(node.pos.clone());
                                nodes.push(node);
                            }
                        }
                        if let Some(token) = self.get() {
                            if token.instr == Instr::Else {
                                self.advance();
                                let mut else_nodes = vec![];
                                while let Some(token) = self.get() {
                                    if token.instr == Instr::End { self.advance(); break }
                                    if let Some(node) = self.next()? {
                                        pos.extend(node.pos.clone());
                                        else_nodes.push(node);
                                    }
                                }
                                let chunk = if else_nodes.len() == 1 {
                                    Box::new(else_nodes[0].clone())
                                } else {
                                    Box::new(Node::new(NodeType::Chunk(else_nodes), pos.clone()))
                                };
                                else_node = Some(chunk);
                            }
                        }
                        let chunk = if nodes.len() == 1 {
                            Box::new(nodes[0].clone())
                        } else {
                            Box::new(Node::new(NodeType::Chunk(nodes), pos.clone()))
                        };
                        Ok(Some(Node::new(NodeType::If(chunk, else_node), pos)))
                    }
                    Instr::Repeat => {
                        self.advance();
                        let mut nodes = vec![];
                        while let Some(token) = self.get() {
                            if token.instr == Instr::End { self.advance(); break }
                            if let Some(node) = self.next()? {
                                pos.extend(node.pos.clone());
                                nodes.push(node);
                            }
                        }
                        let chunk = if nodes.len() == 1 {
                            Box::new(nodes[0].clone())
                        } else {
                            Box::new(Node::new(NodeType::Chunk(nodes), pos.clone()))
                        };
                        Ok(Some(Node::new(NodeType::Repeat(chunk), pos)))
                    }
                    _ => error_pos!(&token.pos, "unexpected {}", token.instr)
                }
            }
            None => Ok(None)
        }
    }
    pub fn parse(&mut self) -> Result<Node, Error> {
        if self.tokens.len() == 0 { return Ok(Node::new(NodeType::Chunk(vec![]), Position::zero())) }
        let mut nodes = vec![];
        let mut pos = self.pos().unwrap().clone();
        while let Some(node) = self.next()? {
            pos.extend(node.pos.clone());
            nodes.push(node);
        }
        Ok(Node::new(NodeType::Chunk(nodes), pos))
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<Node, Error> {
    Parser::new(tokens).parse()
}