use crate::ast::*;
use crate::lexer::{Token, TokenInfo};

pub struct Parser<'a> { tokens: &'a [TokenInfo], pos: usize, pub is_module: bool }

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [TokenInfo]) -> Self { Self { tokens, pos: 0, is_module: false } }
    fn peek(&self) -> Option<Token> { self.tokens.get(self.pos).map(|t| t.token.clone()) }
    fn current_line(&self) -> usize { self.tokens.get(self.pos.saturating_sub(1)).map(|t| t.line).unwrap_or(0) }
    fn consume(&mut self) -> Option<Token> { 
        let t = self.tokens.get(self.pos).map(|t| t.token.clone()); 
        self.pos += 1; 
        t 
    }
    fn skip_newlines(&mut self) { 
        while let Some(Token::Newline) | Some(Token::Semicolon) = self.peek() { self.pos += 1; } 
    }
    
    fn parse_term(&mut self) -> Result<Term, String> {
        let line = self.current_line();
        match self.consume() {
            Some(Token::Int(v)) => Ok(Term::Number(v)),
            Some(Token::Identifier(n)) => if n == "argc" { Ok(Term::ArgC) } else { Ok(Term::Variable(n)) },
            other => Err(format!("Parser Error [Line {}]: Expected number or variable, found {:?}", line, other)),
        }
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, String> {
        let mut left_expr = Expression::Term(self.parse_term()?);
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Star => MathOp::Mul,
                Token::Slash => MathOp::Div,
                Token::Modulo => MathOp::Mod,
                _ => break,
            };
            self.consume();
            let right_term = self.parse_term()?;
            left_expr = Expression::Binary(Box::new(left_expr), op, Box::new(Expression::Term(right_term)));
        }
        Ok(left_expr)
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        let mut left_expr = self.parse_multiplicative()?;
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Plus => MathOp::Add, Token::Minus => MathOp::Sub, _ => break,
            };
            self.consume();
            let right_expr = self.parse_multiplicative()?;
            left_expr = Expression::Binary(Box::new(left_expr), op, Box::new(right_expr));
        }
        Ok(left_expr)
    }

    fn parse_block(&mut self) -> Result<Vec<Node>, String> {
        let line = self.current_line();
        if self.consume() != Some(Token::LBrace) { return Err(format!("Parser Error [Line {}]: Expected '{{'", line)); }
        let mut nodes = Vec::new();
        loop {
            self.skip_newlines();
            if self.peek() == Some(Token::RBrace) { self.consume(); break; }
            if let Some(node) = self.parse_statement()? { nodes.push(node); } else { break; }
        }
        Ok(nodes)
    }

    fn parse_condition(&mut self) -> Result<(Term, CompOp, Term), String> {
        let left = self.parse_term()?;
        let line = self.current_line();
        let op = match self.consume() {
            Some(Token::EqEq) => CompOp::Eq,
            Some(Token::NotEq) => CompOp::NotEq,
            Some(Token::Less) => CompOp::Less,
            Some(Token::LessEq) => CompOp::LessEq,
            Some(Token::Greater) => CompOp::Greater,
            Some(Token::GreaterEq) => CompOp::GreaterEq,
            _ => return Err(format!("Parser Error [Line {}]: Expected comparison operator", line)),
        };
        let right = self.parse_term()?;
        Ok((left, op, right))
    }

    fn parse_statement(&mut self) -> Result<Option<Node>, String> {
        self.skip_newlines();
        let token = match self.consume() {
            Some(t) => t,
            None => return Ok(None),
        };
        let line = self.current_line();

        match token {
            Token::Module => { self.is_module = true; self.parse_statement() }
            Token::Let => {
                let name = if let Some(Token::Identifier(n)) = self.consume() { n } else { return Err(format!("Parser Error [Line {}]: Expected variable name", line)) };
                if self.peek() == Some(Token::Equals) {
                    self.consume(); Ok(Some(Node::VarDeclaration(name, Some(self.parse_expression()?))))
                } else { Ok(Some(Node::VarDeclaration(name, None))) }
            }
            Token::Exec => {
                if let Some(Token::StringLiteral(file)) = self.consume() { Ok(Some(Node::ExecStatement(file))) }
                else { Err(format!("Parser Error [Line {}]: exec requires a string", line)) }
            }
            Token::Identifier(name) => {
                if self.peek() == Some(Token::Equals) { 
                    self.consume(); Ok(Some(Node::Assignment(name, self.parse_expression()?)))
                } else { Ok(None) }
            }
            Token::Input => {
                let name = if let Some(Token::Identifier(n)) = self.consume() { n } else { return Err(format!("Parser Error [Line {}]: Expected variable", line)) };
                Ok(Some(Node::InputStatement(name)))
            }
            Token::If => {
                let mut branches = Vec::new();
                let (l, o, r) = self.parse_condition()?;
                branches.push(IfBranch { left: l, op: o, right: r, block: self.parse_block()? });
                loop {
                    self.skip_newlines();
                    if self.peek() == Some(Token::Elif) {
                        self.consume();
                        let (l, o, r) = self.parse_condition()?;
                        branches.push(IfBranch { left: l, op: o, right: r, block: self.parse_block()? });
                    } else { break; }
                }
                let mut else_block = Vec::new();
                self.skip_newlines();
                if self.peek() == Some(Token::Else) {
                    self.consume(); else_block = self.parse_block()?;
                }
                Ok(Some(Node::IfStatement { branches, else_block }))
            }
            Token::While => {
                let (l, o, r) = self.parse_condition()?;
                let block = self.parse_block()?;
                Ok(Some(Node::WhileStatement(l, o, r, block)))
            }
            Token::Print | Token::Println => {
                let append_newline = token == Token::Println;
                if let Some(Token::StringLiteral(s)) = self.peek() {
                    let text = s.clone(); self.consume(); Ok(Some(Node::PrintString(text, append_newline)))
                } else { Ok(Some(Node::PrintVar(self.parse_term()?, append_newline))) }
            }
            Token::Exit => Ok(Some(Node::ExitStatement(self.parse_term()?))),
            _ => Ok(None),
        }
    }
    
    pub fn parse_all(&mut self) -> Result<Vec<Node>, String> {
        let mut nodes = Vec::new();
        while let Some(node) = self.parse_statement()? { nodes.push(node); }
        Ok(nodes)
    }
}