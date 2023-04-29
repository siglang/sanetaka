mod arguments;
use std::fs;

use analyzer::analyzer::Analyzer;
use clap::Parser as _;
use parser::{tokenizer::Lexer, Parser};

fn main() {
    let args = arguments::Cli::parse();
    let content = fs::read_to_string(args.source).unwrap();

    let lexer = Lexer::new(content.as_str());
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_program();

    // for debugging
    if !ast.errors.is_empty() {
        for error in ast.errors {
            println!("{}", error);
        }
    } else {
        #[allow(unused_variables)]
        let result = Analyzer::new(ast).analyze().unwrap_or_else(|error| {
            println!("{}", error);
            std::process::exit(1);
        });
        // println!("{result:#?}");
    }
}
