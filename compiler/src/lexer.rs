#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Let, If, Elif, Else, While, Exit, Print, Println, Input, Exec, Module,
    Identifier(String), Int(i32), StringLiteral(String),
    Equals, EqEq, NotEq, Less, LessEq, Greater, GreaterEq, Plus, Minus, Star, Slash, Modulo,
    LBrace, RBrace, Newline, Semicolon,
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token: Token,
    pub line: usize,
}

pub fn lex(source: &str) -> Vec<TokenInfo> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    let mut line = 1;
    
    while let Some(&c) = chars.peek() {
        if c == '\n' { 
            tokens.push(TokenInfo { token: Token::Newline, line });
            line += 1;
            chars.next(); 
        } else if c.is_whitespace() { 
            chars.next(); 
        } else if c == '/' {
            chars.next();
            if chars.peek() == Some(&'/') {
                while let Some(&ch) = chars.peek() {
                    if ch == '\n' { break; }
                    chars.next();
                }
            } else {
                tokens.push(TokenInfo { token: Token::Slash, line });
            }
        } else if c == '+' { tokens.push(TokenInfo { token: Token::Plus, line }); chars.next(); }
        else if c == '-' { tokens.push(TokenInfo { token: Token::Minus, line }); chars.next(); }
        else if c == '*' { tokens.push(TokenInfo { token: Token::Star, line }); chars.next(); }
        else if c == '%' { tokens.push(TokenInfo { token: Token::Modulo, line }); chars.next(); }
        else if c == '{' { tokens.push(TokenInfo { token: Token::LBrace, line }); chars.next(); }
        else if c == '}' { tokens.push(TokenInfo { token: Token::RBrace, line }); chars.next(); }
        else if c == '<' {
            chars.next();
            if chars.peek() == Some(&'=') { tokens.push(TokenInfo { token: Token::LessEq, line }); chars.next(); }
            else { tokens.push(TokenInfo { token: Token::Less, line }); }
        }
        else if c == '>' {
            chars.next();
            if chars.peek() == Some(&'=') { tokens.push(TokenInfo { token: Token::GreaterEq, line }); chars.next(); }
            else { tokens.push(TokenInfo { token: Token::Greater, line }); }
        }
        else if c == '=' {
            chars.next();
            if chars.peek() == Some(&'=') { tokens.push(TokenInfo { token: Token::EqEq, line }); chars.next(); }
            else { tokens.push(TokenInfo { token: Token::Equals, line }); }
        }
        else if c == '!' {
            chars.next();
            if chars.peek() == Some(&'=') { tokens.push(TokenInfo { token: Token::NotEq, line }); chars.next(); }
            else { panic!("Lexer Error [Line {}]: Expected '=' after '!'", line); }
        }
        else if c.is_alphabetic() {
            let mut ident = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' { ident.push(chars.next().unwrap()); } 
                else { break; }
            }
            let token = match ident.as_str() {
                "let" => Token::Let, "if" => Token::If, "elif" => Token::Elif,
                "else" => Token::Else, "while" => Token::While, "exit" => Token::Exit,
                "print" => Token::Print, "println" => Token::Println, 
                "input" => Token::Input, "exec" => Token::Exec, "module" => Token::Module,
                "true" => Token::Int(1), "false" => Token::Int(0), // Логічні константи
                _ => Token::Identifier(ident),
            };
            tokens.push(TokenInfo { token, line });
        } else if c.is_digit(10) {
            let mut num_str = String::new();
            let is_hex = c == '0' && chars.clone().nth(1) == Some('x');
            
            if is_hex {
                chars.next(); chars.next();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_hexdigit() { num_str.push(chars.next().unwrap()); } 
                    else { break; }
                }
                tokens.push(TokenInfo { token: Token::Int(i32::from_str_radix(&num_str, 16).unwrap_or(0)), line });
            } else {
                while let Some(&ch) = chars.peek() {
                    if ch.is_digit(10) { num_str.push(chars.next().unwrap()); } 
                    else { break; }
                }
                tokens.push(TokenInfo { token: Token::Int(num_str.parse().unwrap()), line });
            }
        } else if c == '"' {
            chars.next();
            let mut string_val = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == '"' { chars.next(); break; }
                if ch == '\n' { line += 1; }
                
                if ch == '\\' {
                    chars.next();
                    match chars.peek() {
                        Some(&'n') => { string_val.push('\n'); chars.next(); },
                        Some(&'t') => { string_val.push('\t'); chars.next(); },
                        Some(&'"') => { string_val.push('"'); chars.next(); },
                        Some(&'\\') => { string_val.push('\\'); chars.next(); },
                        _ => string_val.push('\\'),
                    }
                } else {
                    string_val.push(chars.next().unwrap());
                }
            }
            tokens.push(TokenInfo { token: Token::StringLiteral(string_val), line });
        } else if c == ';' { tokens.push(TokenInfo { token: Token::Semicolon, line }); chars.next(); }
        else { panic!("Lexer Error [Line {}]: Unknown character: '{}'", line, c); }
    }
    tokens
}