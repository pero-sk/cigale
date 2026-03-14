use std::{env, sync::{Arc, Mutex}};

mod lexer;
use lexer::Lexer;

mod parser;
use parser::Parser;

mod analyser;
use analyser::Analyser;

mod interpreter;
use interpreter::Interpreter;

fn run(file: &str) {

    // resolve source path -- try as-is first (full or relative to cwd)
    let resolved_path = if std::path::Path::new(&file).exists() {
        file
    } else {
        eprintln!("error: cannot find file '{}'", file);
        return;
    };

    let source = match std::fs::read_to_string(&resolved_path) {
        Ok(s) => s,
        Err(e) => { eprintln!("error reading file: {}", e); return; }
    };


    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);

    
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => { eprintln!("Parse error: {}", e); return; }
    };

    // println!("{:#?}", program);

    let mut analyser = Analyser::new();
    let issues = analyser.analyse(&program);

    let mut has_errors = false;
    for issue in issues {
        match issue {
            analyser::AnalysisError::Error(e) => {
                eprintln!("Error: {}", e);
                has_errors = true;
            }
            analyser::AnalysisError::Warning(w) => {
                eprintln!("Warning: {}", w);
            }
        }
    }
    if has_errors {
        return;
    }


    let interpreter = Arc::new(Mutex::new(Interpreter::new()));
    {
        let mut interp = interpreter.lock().unwrap();
        #[cfg(feature = "stdl")]
        {
            interp.self_ref = Some(Arc::downgrade(&interpreter));
        }
        interp.base_dir = std::path::Path::new(&resolved_path)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_string_lossy()
            .to_string();
    }

    match interpreter.lock().unwrap().run(program) {
        Ok(_)  => {}
        Err(e) => { eprintln!("Runtime error: {}", e); }
    }
}

pub fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("usage: cigale <file>");
        return;
    }

    let file = &args[1];
    
    run(file);

}