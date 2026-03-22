mod ast;
mod lexer;
mod parser;
mod compiler;

use std::env;
use std::fs;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 { 
        println!("Usage: {} <file.jds>", args[0]);
        return; 
    }
    
    let input_file = match args.iter().find(|a| a.ends_with(".jds")) {
        Some(f) => f,
        None => {
            eprintln!("Error: File with .jds extension not found");
            return;
        }
    };
    
    let output_name = input_file.trim_end_matches(".jds");

    let source = match fs::read_to_string(input_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read file {}: {}", input_file, e);
            return;
        }
    };

    let tokens = lexer::lex(&source);
    
    let mut parser = parser::Parser::new(&tokens);
    let ast = match parser.parse_all() {
        Ok(nodes) => nodes,
        Err(e) => {
            eprintln!("Syntax Error: {}", e);
            return;
        }
    };
    let is_module = parser.is_module; 
    let asm = compiler::Compiler::new(is_module).generate(ast, is_module);

    if is_module {
        fs::write("mod.asm", &asm).unwrap();
        let status = Command::new("nasm")
            .args(["-f", "bin", "mod.asm", "-o", &format!("{}.bin", output_name)])
            .status()
            .expect("Error: NASM is not installed or not in PATH");
            
        if status.success() {
            let _ = fs::remove_file("mod.asm");
            println!("Module successfully compiled: {}.bin", output_name);
        } else {
            eprintln!("Module assembly error!");
        }
    } else {
        fs::write("output.asm", &asm).unwrap();
        let nasm_status = Command::new("nasm")
            .args(["-f", "elf64", "output.asm", "-o", "output.o"])
            .status()
            .expect("Error: NASM is not installed or not in PATH");

        if nasm_status.success() {
            let ld_status = Command::new("ld")
                .args(["-s", "-n", "--no-warn-rwx-segments", "output.o", "-o", output_name])
                .status()
                .expect("Error: LD (linker) not found");
                
            if ld_status.success() {
                let _ = fs::remove_file("output.asm");
                let _ = fs::remove_file("output.o");
                println!("Program successfully compiled: ./{}", output_name);
            } else {
                eprintln!("Linking error!");
            }
        } else {
            eprintln!("Assembly error!");
        }
    }
}