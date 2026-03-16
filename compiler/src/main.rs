use std::env;
use std::fs;
use std::process::Command;
use std::collections::HashSet;

// ==========================================
// 1. ЛЕКСЕР
// ==========================================
#[derive(Debug, PartialEq, Clone)]
enum Token {
    Let, If, Elif, Else, While, Exit, Print, Input, Exec, Module,
    Identifier(String), Int(i32), StringLiteral(String),
    Equals, EqEq, Less, Greater, Plus, Minus, Star, Slash,
    LBrace, RBrace, Newline, Semicolon,
}

fn lex(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    
    while let Some(&c) = chars.peek() {
        if c == '\n' { 
            tokens.push(Token::Newline); chars.next(); 
        } else if c.is_whitespace() { 
            chars.next(); 
        } else if c == '/' {
            chars.next();
            // ДОДАНО: Підтримка коментарів //
            if chars.peek() == Some(&'/') {
                while let Some(&ch) = chars.peek() {
                    if ch == '\n' { break; }
                    chars.next();
                }
            } else {
                tokens.push(Token::Slash);
            }
        } else if c == '+' { tokens.push(Token::Plus); chars.next(); }
        else if c == '-' { tokens.push(Token::Minus); chars.next(); }
        else if c == '*' { tokens.push(Token::Star); chars.next(); }
        else if c == '{' { tokens.push(Token::LBrace); chars.next(); }
        else if c == '}' { tokens.push(Token::RBrace); chars.next(); }
        else if c == '<' { tokens.push(Token::Less); chars.next(); }
        else if c == '>' { tokens.push(Token::Greater); chars.next(); }
        else if c == '=' {
            chars.next();
            if chars.peek() == Some(&'=') { tokens.push(Token::EqEq); chars.next(); }
            else { tokens.push(Token::Equals); }
        }
        else if c.is_alphabetic() {
            let mut ident = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' { ident.push(chars.next().unwrap()); } 
                else { break; }
            }
            match ident.as_str() {
                "let" => tokens.push(Token::Let), "if" => tokens.push(Token::If),
                "elif" => tokens.push(Token::Elif), "else" => tokens.push(Token::Else),
                "while" => tokens.push(Token::While), "exit" => tokens.push(Token::Exit),
                "print" => tokens.push(Token::Print), "input" => tokens.push(Token::Input),
                "exec" => tokens.push(Token::Exec),  "module" => tokens.push(Token::Module),
                _ => tokens.push(Token::Identifier(ident)),
            }
        } else if c.is_digit(10) {
            let mut num_str = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_digit(10) { num_str.push(chars.next().unwrap()); } 
                else { break; }
            }
            tokens.push(Token::Int(num_str.parse().unwrap()));
        } else if c == '"' {
            chars.next(); // Скіпаємо першу "
            let mut string_val = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == '"' { chars.next(); break; }
                
                // ДОДАНО: Екранування символів (наприклад \n)
                if ch == '\\' {
                    chars.next();
                    match chars.peek() {
                        Some(&'n') => { string_val.push('\n'); chars.next(); },
                        Some(&'t') => { string_val.push('\t'); chars.next(); },
                        Some(&'"') => { string_val.push('"'); chars.next(); },
                        _ => string_val.push('\\'),
                    }
                } else {
                    string_val.push(chars.next().unwrap());
                }
            }
            tokens.push(Token::StringLiteral(string_val));
        } else if c == ';' { tokens.push(Token::Semicolon); chars.next(); }
        else { panic!("Lexer: Unknown character: '{}'", c); }
    }
    tokens
}

// ==========================================
// 2. ПАРСЕР
// ==========================================
#[derive(Debug, Clone)]
enum Term { Number(i32), Variable(String), ArgC }

#[derive(Debug, Clone)]
enum MathOp { Add, Sub, Mul, Div }

#[derive(Debug, Clone)]
enum CompOp { Eq, Less, Greater }

// ДОДАНО: Рекурсивне AST для виразів (тепер працює a + b * c, хоч і без пріоритетів поки що)
#[derive(Debug, Clone)]
enum Expression { 
    Term(Term), 
    Binary(Box<Expression>, MathOp, Box<Expression>) 
}

#[derive(Debug, Clone)]
struct IfBranch {
    left: Term,
    op: CompOp,
    right: Term,
    block: Vec<Node>,
}

#[derive(Debug, Clone)]
enum Node {
    VarDeclaration(String, Option<Expression>),
    Assignment(String, Expression),
    ExitStatement(Term),
    PrintString(String),
    PrintVar(Term),
    IfStatement {
        branches: Vec<IfBranch>,
        else_block: Vec<Node>,
    },
    WhileStatement(Term, CompOp, Term, Vec<Node>),
    InputStatement(String),
    ExecStatement(String),
}

struct Parser<'a> { tokens: &'a [Token], pos: usize, is_module: bool }

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self { Self { tokens, pos: 0, is_module: false } }
    fn peek(&self) -> Option<Token> { self.tokens.get(self.pos).cloned() }
    fn consume(&mut self) -> Option<Token> { let t = self.tokens.get(self.pos).cloned(); self.pos += 1; t }
    fn skip_newlines(&mut self) { while let Some(Token::Newline) | Some(Token::Semicolon) = self.peek() { self.pos += 1; } }
    
    fn parse_term(&mut self) -> Term {
        match self.consume() {
            Some(Token::Int(v)) => Term::Number(v),
            Some(Token::Identifier(n)) => {
                if n == "argc" { Term::ArgC } else { Term::Variable(n) }
            },
            other => panic!("Parser: Expected number or variable, found {:?}", other),
        }
    }

    // ДОДАНО: Обробка множення та ділення (вищий пріоритет)
    fn parse_multiplicative(&mut self) -> Expression {
        let mut left_expr = Expression::Term(self.parse_term());
        
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Star => MathOp::Mul,
                Token::Slash => MathOp::Div,
                _ => break, // Якщо не * або /, виходимо з циклу
            };
            
            self.consume(); // З'їдаємо оператор
            let right_term = self.parse_term();
            left_expr = Expression::Binary(Box::new(left_expr), op, Box::new(Expression::Term(right_term)));
        }
        
        left_expr
    }

    // ВИПРАВЛЕНО: Тепер обробляє додавання/віднімання і викликає parse_multiplicative
    fn parse_expression(&mut self) -> Expression {
        let mut left_expr = self.parse_multiplicative();
        
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Plus => MathOp::Add,
                Token::Minus => MathOp::Sub,
                _ => break, // Якщо не + або -, виходимо
            };
            
            self.consume(); // З'їдаємо оператор
            let right_expr = self.parse_multiplicative();
            left_expr = Expression::Binary(Box::new(left_expr), op, Box::new(right_expr));
        }
        
        left_expr
    }

    fn parse_block(&mut self) -> Vec<Node> {
        if self.consume() != Some(Token::LBrace) { panic!("Parser: Expected '{{'"); }
        let mut nodes = Vec::new();
        loop {
            self.skip_newlines();
            if self.peek() == Some(Token::RBrace) { self.consume(); break; }
            if let Some(node) = self.parse_statement() { nodes.push(node); } else { break; }
        }
        nodes
    }

    fn parse_condition(&mut self) -> (Term, CompOp, Term) {
        let left = self.parse_term();
        let op = match self.consume() {
            Some(Token::EqEq) => CompOp::Eq,
            Some(Token::Less) => CompOp::Less,
            Some(Token::Greater) => CompOp::Greater,
            _ => panic!("Parser: Expected comparison operator (==, <, >)"),
        };
        let right = self.parse_term();
        (left, op, right)
    }

    fn parse_statement(&mut self) -> Option<Node> {
        self.skip_newlines();
        let token = self.consume()?;
        match token {
            Token::Module => { self.is_module = true; self.parse_statement() }
            Token::Let => {
                let name = if let Some(Token::Identifier(n)) = self.consume() { n } else { panic!("Expected variable name after let") };
                if self.peek() == Some(Token::Equals) {
                    self.consume();
                    Some(Node::VarDeclaration(name, Some(self.parse_expression())))
                } else {
                    Some(Node::VarDeclaration(name, None))
                }
            }
            Token::Exec => {
                if let Some(Token::StringLiteral(file)) = self.consume() { Some(Node::ExecStatement(file)) }
                else { panic!("exec requires a string (file name)") }
            }
            Token::Identifier(name) => {
                if self.peek() == Some(Token::Equals) { 
                    self.consume(); 
                    Some(Node::Assignment(name, self.parse_expression()))
                } else { None }
            }
            Token::Input => {
                let name = if let Some(Token::Identifier(n)) = self.consume() { n } else { panic!("Expected variable after input") };
                Some(Node::InputStatement(name))
            }
            Token::If => {
                let mut branches = Vec::new();
                let (l, o, r) = self.parse_condition();
                branches.push(IfBranch { left: l, op: o, right: r, block: self.parse_block() });
                
                loop {
                    self.skip_newlines();
                    if self.peek() == Some(Token::Elif) {
                        self.consume();
                        let (l, o, r) = self.parse_condition();
                        branches.push(IfBranch { left: l, op: o, right: r, block: self.parse_block() });
                    } else { break; }
                }

                let mut else_block = Vec::new();
                self.skip_newlines();
                if self.peek() == Some(Token::Else) {
                    self.consume();
                    else_block = self.parse_block();
                }
                Some(Node::IfStatement { branches, else_block })
            }
            Token::While => {
                let (l, o, r) = self.parse_condition();
                let block = self.parse_block();
                Some(Node::WhileStatement(l, o, r, block))
            }
            Token::Print => {
                if let Some(Token::StringLiteral(s)) = self.peek() {
                    let text = s.clone(); self.consume(); Some(Node::PrintString(text))
                } else { Some(Node::PrintVar(self.parse_term())) }
            }
            Token::Exit => Some(Node::ExitStatement(self.parse_term())),
            _ => None,
        }
    }
    fn parse_all(&mut self) -> Vec<Node> {
        let mut nodes = Vec::new();
        while let Some(node) = self.parse_statement() { nodes.push(node); }
        nodes
    }
}

// ==========================================
// 3. ГЕНЕРАТОР КОДУ (COMPILER)
// ==========================================

// ВИПРАВЛЕНО: Стандартна бібліотека винесена окремо для чистоти коду
const STDLIB_ASM: &str = r#"
_print_rax:
    test rax, rax
    jns ._print_positive
    push rax
    mov rax, 1
    mov rdi, 1
    lea rsi, [rel minus_sign]
    mov rdx, 1
    syscall
    pop rax
    neg rax
._print_positive:
    lea rcx, [rel digit_space]
    mov rbx, 10
    mov [rcx], rbx
    inc rcx
    mov [rel digit_space_pos], rcx
_print_rax_loop:
    mov rdx, 0
    mov rbx, 10
    div rbx
    push rax
    add rdx, 48
    mov rcx, [rel digit_space_pos]
    mov [rcx], dl
    inc rcx
    mov [rel digit_space_pos], rcx
    pop rax
    test rax, rax
    jnz _print_rax_loop
_print_rax_loop2:
    mov rcx, [rel digit_space_pos]
    mov rax, 1
    mov rdi, 1
    mov rsi, rcx
    mov rdx, 1
    syscall
    mov rcx, [rel digit_space_pos]
    dec rcx
    mov [rel digit_space_pos], rcx
    lea rdx, [rel digit_space]
    cmp rcx, rdx
    jge _print_rax_loop2
    ret

_read_int:
    mov rax, 0
    mov rdi, 0
    lea rsi, [rel input_buffer]
    mov rdx, 32
    syscall
    lea rsi, [rel input_buffer]
    mov rax, 0
    mov rcx, 0
    movzx rbx, byte [rsi]
    cmp rbx, 45
    jne _atoi_loop
    inc rcx
    inc rsi
_atoi_loop:
    movzx rbx, byte [rsi]
    cmp rbx, 10
    je _atoi_done
    cmp rbx, 0
    je _atoi_done
    sub rbx, 48
    imul rax, 10
    add rax, rbx
    inc rsi
    jmp _atoi_loop
_atoi_done:
    test rcx, rcx
    jz _atoi_ret
    neg rax
_atoi_ret:
    ret

_load_and_run:
    push rbx
    push r12
    push r13

    mov rax, 2
    mov rsi, 0
    syscall
    test rax, rax
    js .err
    mov r12, rax

    mov rax, 9
    xor rdi, rdi
    mov rsi, 4096
    mov rdx, 7
    mov r10, 34 
    mov r8, -1
    xor r9, r9
    syscall
    mov r13, rax

    mov rdi, r12
    mov rsi, r13
    mov rdx, 4096
    xor rax, rax
    syscall

    mov rdi, r12
    mov rax, 3
    syscall

    call r13

    mov rdi, r13
    mov rsi, 4096
    mov rax, 11
    syscall
.err:
    pop r13
    pop r12
    pop rbx
    ret
"#;

struct Compiler {
    bss: String, data: String, text: String,
    str_count: usize, if_count: usize, while_count: usize,
    vars: HashSet<String>,
}

impl Compiler {
    fn new(is_module: bool) -> Self {
        let entry = if is_module { "" } else { "global _start\n_start:\n    mov rax, [rsp]\n    mov [rel argc_val], rax\n" };
        Self {
            bss: String::from("section .bss\n    digit_space resb 100\n    digit_space_pos resb 8\n    input_buffer resb 32\n    argc_val resq 1\n"),
            data: String::from("section .data\n    minus_sign db 45\n"),
            text: format!("[bits 64]\ndefault rel\nsection .text\n{}", entry),
            str_count: 0, if_count: 0, while_count: 0,
            vars: HashSet::new(),
        }
    }

    fn ensure_var(&mut self, name: &str, is_module: bool) {
        if !self.vars.contains(name) {
            self.vars.insert(name.to_string());
            if is_module {
                self.data.push_str(&format!("    {} dq 0\n", name));
            } else {
                self.bss.push_str(&format!("    {} resq 1\n", name));
            }
        }
    }

    fn load_term(&self, term: &Term, reg: &str) -> String {
        match term {
            Term::Number(val) => format!("    mov {}, {}\n", reg, val),
            Term::ArgC => format!("    mov {}, [rel argc_val]\n", reg),
            Term::Variable(name) => format!("    mov {}, qword [rel {}]\n", reg, name),
        }
    }

    // ВИПРАВЛЕНО: Рекурсивна генерація виразів за допомогою стеку
    fn eval_expr(&self, expr: &Expression) -> String {
        let mut code = String::new();
        match expr {
            Expression::Term(term) => {
                code.push_str(&self.load_term(term, "rax"));
            },
            Expression::Binary(left, op, right) => {
                // Рахуємо ліву частину, кладемо в стек
                code.push_str(&self.eval_expr(left));
                code.push_str("    push rax\n");
                
                // Рахуємо праву частину (вона буде в rax)
                code.push_str(&self.eval_expr(right));
                code.push_str("    mov rbx, rax\n"); // права частина в rbx
                code.push_str("    pop rax\n");      // ліва частина знову в rax

                match op {
                    MathOp::Add => code.push_str("    add rax, rbx\n"),
                    MathOp::Sub => code.push_str("    sub rax, rbx\n"),
                    MathOp::Mul => code.push_str("    imul rax, rbx\n"),
                    MathOp::Div => code.push_str("    cqo\n    idiv rbx\n"),
                }
            }
        }
        code
    }

    fn compile_nodes(&mut self, nodes: Vec<Node>, is_module: bool) {
        for node in nodes {
            match node {
                Node::VarDeclaration(name, expr_opt) => {
                    self.ensure_var(&name, is_module);
                    if let Some(expr) = expr_opt {
                        self.text.push_str(&self.eval_expr(&expr));
                        self.text.push_str(&format!("    mov qword [rel {}], rax\n", name));
                    }
                }
                Node::Assignment(name, expr) => {
                    self.ensure_var(&name, is_module);
                    self.text.push_str(&self.eval_expr(&expr));
                    self.text.push_str(&format!("    mov qword [rel {}], rax\n", name));
                }
                Node::InputStatement(name) => {
                    self.ensure_var(&name, is_module);
                    self.text.push_str("    call _read_int\n");
                    self.text.push_str(&format!("    mov qword [rel {}], rax\n", name));
                }
                Node::PrintString(text) => {
                    let lbl = format!("str_{}", self.str_count); self.str_count += 1;
                    // Оскільки ми вже підтримуємо \n в лексері, NASM має приймати '`' (backticks) для escape-послідовностей
                    self.data.push_str(&format!("    {} db `{}`, 10\n    {}_len equ $ - {}\n", lbl, text, lbl, lbl));
                    self.text.push_str(&format!("    mov rax, 1\n    mov rdi, 1\n    lea rsi, [rel {}]\n    mov rdx, {}_len\n    syscall\n", lbl, lbl));
                }
                Node::PrintVar(term) => {
                    self.text.push_str(&self.load_term(&term, "rax"));
                    self.text.push_str("    call _print_rax\n");
                }
                Node::ExecStatement(filename) => {
                    let lbl = format!("file_{}", self.str_count); self.str_count += 1;
                    let fixed_path = if filename.starts_with('/') { filename } else { format!("./{}", filename) };
                    self.data.push_str(&format!("    {} db `{}`, 0\n", lbl, fixed_path));
                    self.text.push_str(&format!("    lea rdi, [rel {}]\n    call _load_and_run\n", lbl));
                }
                Node::ExitStatement(term) => {
                    self.text.push_str(&self.load_term(&term, "rdi"));
                    self.text.push_str("    mov rax, 60\n    syscall\n");
                }
                Node::IfStatement { branches, else_block } => {
                    let end_label = format!(".L_if_chain_end_{}", self.if_count);
                    let my_if_id = self.if_count;
                    self.if_count += 1;

                    for (idx, branch) in branches.iter().enumerate() {
                        let next_branch_label = format!(".L_branch_{}_{}", my_if_id, idx + 1);
                        
                        self.text.push_str(&self.load_term(&branch.left, "rax"));
                        self.text.push_str(&self.load_term(&branch.right, "rbx"));
                        self.text.push_str("    cmp rax, rbx\n");
                        
                        let jump_target = if idx == branches.len() - 1 && else_block.is_empty() {
                            &end_label
                        } else {
                            &next_branch_label
                        };

                        match branch.op { 
                            CompOp::Eq => self.text.push_str(&format!("    jne {}\n", jump_target)), 
                            CompOp::Less => self.text.push_str(&format!("    jge {}\n", jump_target)), 
                            CompOp::Greater => self.text.push_str(&format!("    jle {}\n", jump_target)) 
                        }
                        
                        self.compile_nodes(branch.block.clone(), is_module);
                        self.text.push_str(&format!("    jmp {}\n", end_label));
                        self.text.push_str(&format!("{}:\n", next_branch_label));
                    }

                    if !else_block.is_empty() {
                        self.compile_nodes(else_block, is_module);
                    }
                    self.text.push_str(&format!("{}:\n", end_label));
                }
                Node::WhileStatement(l, op, r, b) => {
                    let start = format!(".L_wh_s_{}", self.while_count);
                    let end = format!(".L_wh_e_{}", self.while_count); self.while_count += 1;
                    self.text.push_str(&format!("{}:\n", start));
                    self.text.push_str(&self.load_term(&l, "rax"));
                    self.text.push_str(&self.load_term(&r, "rbx"));
                    self.text.push_str("    cmp rax, rbx\n");
                    match op { CompOp::Eq => self.text.push_str(&format!("    jne {}\n", end)), CompOp::Less => self.text.push_str(&format!("    jge {}\n", end)), CompOp::Greater => self.text.push_str(&format!("    jle {}\n", end)) }
                    self.compile_nodes(b, is_module); self.text.push_str(&format!("    jmp {}\n{}:\n", start, end));
                }
            }
        }
    }

    fn generate(mut self, nodes: Vec<Node>, is_module: bool) -> String {
        self.compile_nodes(nodes, is_module);
        
        if is_module { 
            self.text.push_str("    ret\n"); 
        } else { 
            self.text.push_str("    mov rax, 60\n    mov rdi, 0\n    syscall\n"); 
        }

        self.text.push_str(STDLIB_ASM);
        format!("{}\n{}\n{}", self.text, self.data, self.bss)
    }
}

// ==========================================
// 4. MAIN ENTRY POINT
// ==========================================
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 { 
        eprintln!("Usage: {} <file.jds>", args[0]);
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

    let tokens = lex(&source);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse_all();
    let is_module = parser.is_module; 
    let asm = Compiler::new(is_module).generate(ast, is_module);

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