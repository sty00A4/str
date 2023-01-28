use std::fs;

use crate::lexer::Position;

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    msg: String,
    pos: Option<Position>
}
impl Error {
    pub fn new(msg: String, pos: Option<Position>) -> Self { Self { msg, pos } }
    pub fn display(&self, path: &String) -> String {
        let mut err = format!("ERROR: {}", self.msg);
        if let Some(pos) = &self.pos {
            err.push_str(pos.to_string().as_str());
            if let Ok(text) = fs::read_to_string(path) {
                let lines: Vec<&str> = text.lines().collect();
                if let Some(slice) = lines.get(pos.ln.clone()) {
                    for line in slice.to_vec() {
                        err.push_str(line);
                    }
                }
            }
        }
        err
    }
    pub fn display_text(&self, path: &String, text: String) -> String {
        let mut err = format!("ERROR: {}", self.msg);
        if let Some(pos) = &self.pos {
            err.push_str(" - ");
            err.push_str(path.as_str());
            err.push(':');
            err.push_str(pos.to_string().as_str());
            err.push('\n');
            let lines: Vec<&str> = text.lines().collect();
            if let Some(slice) = lines.get(pos.ln.clone()) {
                for line in slice.to_vec() {
                    err.push_str(line);
                }
            }
        }
        err
    }
}