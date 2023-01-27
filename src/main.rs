use std::{env, process::exit, io::{stdout, Write, stdin}};

mod lexer;
mod run;

#[macro_export]
macro_rules! error {
    ($msg:expr, $($s:expr),*) => {
        Err(format!($msg, $($s),*))
    };
    ($msg:expr) => {
        Err(format!($msg))
    };
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut args = args.iter();
    args.next();
    match args.next() {
        Some(arg) => { eprintln!("unrecognized argument {arg:?}"); exit(1) }
        None => {
            let mut program = run::Program::new();
            loop {
                let mut input = String::new();
                print!("> ");
                let _ = stdout().flush();
                let _ = stdin().read_line(&mut input);
                match lexer::lex(input) {
                    Ok(instrs) => match program.run(instrs) {
                        Ok(_) => println!("{}", program.stack),
                        Err(e) => { eprintln!("{e}") }
                    }
                    Err(e) => { eprintln!("{e}") }
                }
            }
        }
    }
}