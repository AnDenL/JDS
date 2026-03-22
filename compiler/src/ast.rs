#[derive(Debug, Clone)]
pub enum Term { Number(i32), Variable(String), ArgC }

#[derive(Debug, Clone)]
pub enum MathOp { Add, Sub, Mul, Div, Mod }

#[derive(Debug, Clone)]
pub enum CompOp { Eq, NotEq, Less, LessEq, Greater, GreaterEq }

#[derive(Debug, Clone)]
pub enum Expression { 
    Term(Term), 
    Binary(Box<Expression>, MathOp, Box<Expression>) 
}

#[derive(Debug, Clone)]
pub struct IfBranch {
    pub left: Term, pub op: CompOp, pub right: Term, pub block: Vec<Node>,
}

#[derive(Debug, Clone)]
pub enum Node {
    VarDeclaration(String, Option<Expression>),
    Assignment(String, Expression),
    ExitStatement(Term),
    PrintString(String, bool),
    PrintVar(Term, bool),
    IfStatement { branches: Vec<IfBranch>, else_block: Vec<Node> },
    WhileStatement(Term, CompOp, Term, Vec<Node>),
    InputStatement(String),
    ExecStatement(String),
}