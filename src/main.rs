#![allow(unused)]
use std::{env, process::exit, io::{stdout, Write, stdin}, fs};
use run::Program;

mod error;
mod lexer;
mod parser;
mod value;
mod run;

#[macro_export]
macro_rules! error_pos {
    ($pos:expr, $msg:expr, $($s:expr),*) => {
        Err(error::Error::new(format!($msg, $($s),*), Some($pos.clone())))
    };
    ($pos:expr, $msg:expr) => {
        Err(error::Error::new(format!($msg), Some($pos.clone())))
    };
}
#[macro_export]
macro_rules! error_no_pos {
    ($msg:expr, $($s:expr),*) => {
        Err(error::Error::new(format!($msg, $($s),*), None))
    };
    ($msg:expr) => {
        Err(error::Error::new(format!($msg), None))
    };
}

fn run(program: &mut Program, path: &String, text: String) {
    match lexer::lex(text.clone()) {
        Ok(tokens) => match parser::parse(tokens) {
            Ok(nodes) => match program.run(nodes) {
                Ok(_) => println!("{}", program.stack),
                Err(e) => { eprintln!("{}\n{}", program.stack, e.display_text(path, text)) }
            }
            Err(e) => { eprintln!("{}", e.display_text(path, text)) }
        }
        Err(e) => { eprintln!("{}", e.display_text(path, text)) }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut args = args.iter();
    args.next();
    match args.next() {
        Some(path) => match fs::read_to_string(path) {
            Ok(text) => {
                let mut program = Program::std_program();
                run(&mut program, path, text);
            }
            Err(e) => { eprintln!("error occurd while reading the file {path:?}: {e}"); exit(1) }
        }
        None => {
            let mut program = Program::std_program();
            let path = &"<stdin>".to_string();
            loop {
                let mut input = String::new();
                print!("> ");
                let _ = stdout().flush();
                let _ = stdin().read_line(&mut input);
                run(&mut program, path, input);
                println!();
            }
        }
    }
}