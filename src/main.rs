#![allow(unused)]
use std::{env, process::exit, io::{stdout, Write, stdin}};

mod error;
mod lexer;
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

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut args = args.iter();
    args.next();
    match args.next() {
        Some(arg) => { eprintln!("unrecognized argument {arg:?}"); exit(1) }
        None => {
            let mut program = run::Program::std_program();
            let path = &"<stdin>".to_string();
            loop {
                let mut input = String::new();
                print!("> ");
                let _ = stdout().flush();
                let _ = stdin().read_line(&mut input);
                match lexer::lex(input.clone()) {
                    Ok(tokens) => match program.run(tokens) {
                        Ok(_) => println!("{}", program.stack),
                        Err(e) => { eprintln!("{}\n{}", program.stack, e.display_text(path, input)) }
                    }
                    Err(e) => { eprintln!("{}", e.display_text(path, input)) }
                }
                println!();
            }
        }
    }
}