use crate::ast::{Node, Expression, Term, MathOp, CompOp};
use std::collections::HashSet;

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

_print_newline:
    mov rax, 1
    mov rdi, 1
    lea rsi, [rel newline_char]
    mov rdx, 1
    syscall
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
    jne ._atoi_loop
    inc rcx
    inc rsi
._atoi_loop:
    movzx rbx, byte [rsi]
    cmp rbx, 10
    je ._atoi_done
    cmp rbx, 0
    je ._atoi_done
    sub rbx, 48
    imul rax, 10
    add rax, rbx
    inc rsi
    jmp ._atoi_loop
._atoi_done:
    test rcx, rcx
    jz ._atoi_ret
    neg rax
._atoi_ret:
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

pub struct Compiler {
    bss: String, data: String, text: String,
    str_count: usize, if_count: usize, while_count: usize,
    vars: HashSet<String>,
}

impl Compiler {
    pub fn new(is_module: bool) -> Self {
        let entry = if is_module { "" } else { "global _start\n_start:\n    mov rax, [rsp]\n    mov [rel argc_val], rax\n" };
        Self {
            bss: String::from("section .bss\n    digit_space resb 100\n    digit_space_pos resb 8\n    input_buffer resb 32\n    argc_val resq 1\n"),
            data: String::from("section .data\n    minus_sign db 45\n    newline_char db 10\n"),
            text: format!("[bits 64]\ndefault rel\nsection .text\n{}", entry),
            str_count: 0, if_count: 0, while_count: 0,
            vars: HashSet::new(),
        }
    }

    fn ensure_var(&mut self, name: &str, is_module: bool) {
        if !self.vars.contains(name) {
            self.vars.insert(name.to_string());
            if is_module { self.data.push_str(&format!("    {} dq 0\n", name)); } 
            else { self.bss.push_str(&format!("    {} resq 1\n", name)); }
        }
    }

    fn load_term(&self, term: &Term, reg: &str) -> String {
        match term {
            Term::Number(val) => format!("    mov {}, {}\n", reg, val),
            Term::ArgC => format!("    mov {}, [rel argc_val]\n", reg),
            Term::Variable(name) => format!("    mov {}, qword [rel {}]\n", reg, name),
        }
    }

    fn eval_expr(&self, expr: &Expression) -> String {
        let mut code = String::new();
        match expr {
            Expression::Term(term) => { code.push_str(&self.load_term(term, "rax")); },
            Expression::Binary(left, op, right) => {
                code.push_str(&self.eval_expr(left));
                code.push_str("    push rax\n");
                code.push_str(&self.eval_expr(right));
                code.push_str("    mov rbx, rax\n    pop rax\n");

                match op {
                    MathOp::Add => code.push_str("    add rax, rbx\n"),
                    MathOp::Sub => code.push_str("    sub rax, rbx\n"),
                    MathOp::Mul => code.push_str("    imul rax, rbx\n"),
                    MathOp::Div => code.push_str("    cqo\n    idiv rbx\n"),
                    MathOp::Mod => code.push_str("    cqo\n    idiv rbx\n    mov rax, rdx\n"),
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
                Node::PrintString(text, append_newline) => {
                    let lbl = format!("str_{}", self.str_count); self.str_count += 1;
                    let mut bytes: Vec<String> = text.bytes().map(|b| b.to_string()).collect();
                    if append_newline { bytes.push("10".to_string()); }
                    let bytes_str = if bytes.is_empty() { "0".to_string() } else { bytes.join(", ") };
                    let len = bytes.len().max(1);

                    self.data.push_str(&format!("    {} db {}\n", lbl, bytes_str));
                    self.text.push_str(&format!("    mov rax, 1\n    mov rdi, 1\n    lea rsi, [rel {}]\n    mov rdx, {}\n    syscall\n", lbl, len));
                }
                Node::PrintVar(term, append_newline) => {
                    self.text.push_str(&self.load_term(&term, "rax"));
                    self.text.push_str("    call _print_rax\n");
                    if append_newline { self.text.push_str("    call _print_newline\n"); }
                }
                Node::ExecStatement(filename) => {
                    let lbl = format!("file_{}", self.str_count); self.str_count += 1;
                    let fixed_path = if filename.starts_with('/') { filename } else { format!("./{}", filename) };
                    let bytes_str = fixed_path.bytes().map(|b| b.to_string()).collect::<Vec<_>>().join(", ");
                    self.data.push_str(&format!("    {} db {}, 0\n", lbl, bytes_str));
                    self.text.push_str(&format!("    lea rdi, [rel {}]\n    call _load_and_run\n", lbl));
                }
                Node::ExitStatement(term) => {
                    self.text.push_str(&self.load_term(&term, "rdi"));
                    self.text.push_str("    mov rax, 60\n    syscall\n");
                }
                Node::IfStatement { branches, else_block } => {
                    let end_label = format!(".L_if_end_{}", self.if_count);
                    let my_if_id = self.if_count;
                    self.if_count += 1;

                    for (idx, branch) in branches.iter().enumerate() {
                        let next_branch_label = format!(".L_br_{}_{}", my_if_id, idx + 1);
                        self.text.push_str(&self.load_term(&branch.left, "rax"));
                        self.text.push_str(&self.load_term(&branch.right, "rbx"));
                        self.text.push_str("    cmp rax, rbx\n");
                        
                        let jump_target = if idx == branches.len() - 1 && else_block.is_empty() { &end_label } else { &next_branch_label };
                        match branch.op { 
                            CompOp::Eq => self.text.push_str(&format!("    jne {}\n", jump_target)), 
                            CompOp::NotEq => self.text.push_str(&format!("    je {}\n", jump_target)), 
                            CompOp::Less => self.text.push_str(&format!("    jge {}\n", jump_target)), 
                            CompOp::LessEq => self.text.push_str(&format!("    jg {}\n", jump_target)), 
                            CompOp::Greater => self.text.push_str(&format!("    jle {}\n", jump_target)), 
                            CompOp::GreaterEq => self.text.push_str(&format!("    jl {}\n", jump_target)), 
                        }
                        self.compile_nodes(branch.block.clone(), is_module);
                        self.text.push_str(&format!("    jmp {}\n{}:\n", end_label, next_branch_label));
                    }
                    if !else_block.is_empty() { self.compile_nodes(else_block, is_module); }
                    self.text.push_str(&format!("{}:\n", end_label));
                }
                Node::WhileStatement(l, op, r, b) => {
                    let start = format!(".L_wh_s_{}", self.while_count);
                    let end = format!(".L_wh_e_{}", self.while_count); self.while_count += 1;
                    self.text.push_str(&format!("{}:\n", start));
                    self.text.push_str(&self.load_term(&l, "rax"));
                    self.text.push_str(&self.load_term(&r, "rbx"));
                    self.text.push_str("    cmp rax, rbx\n");
                    match op { 
                        CompOp::Eq => self.text.push_str(&format!("    jne {}\n", end)), 
                        CompOp::NotEq => self.text.push_str(&format!("    je {}\n", end)), 
                        CompOp::Less => self.text.push_str(&format!("    jge {}\n", end)), 
                        CompOp::LessEq => self.text.push_str(&format!("    jg {}\n", end)), 
                        CompOp::Greater => self.text.push_str(&format!("    jle {}\n", end)), 
                        CompOp::GreaterEq => self.text.push_str(&format!("    jl {}\n", end)), 
                    }
                    self.compile_nodes(b, is_module); 
                    self.text.push_str(&format!("    jmp {}\n{}:\n", start, end));
                }
            }
        }
    }

    pub fn generate(mut self, nodes: Vec<Node>, is_module: bool) -> String {
        self.compile_nodes(nodes, is_module);
        if is_module { self.text.push_str("    ret\n"); } 
        else { self.text.push_str("    mov rax, 60\n    mov rdi, 0\n    syscall\n"); }
        self.text.push_str(STDLIB_ASM);
        format!("{}\n{}\n{}", self.text, self.data, self.bss)
    }
}