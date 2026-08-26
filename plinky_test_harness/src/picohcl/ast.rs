use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Document {
    pub contents: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement<E = Expression> {
    Assignment(AssignmentStatement<E>),
    Block(BlockStatement<E>),
}

pub type ResolvedStatement = Statement<ResolvedExpression>;

#[derive(Debug, Clone)]
pub struct AssignmentStatement<E = Expression> {
    pub key: String,
    pub value: E,
}

#[derive(Debug, Clone)]
pub struct BlockStatement<E = Expression> {
    pub kind: String,
    pub name: Option<String>,
    pub contents: Vec<Statement<E>>,
}

#[derive(Debug, Clone)]
pub enum Expression {
    String(String),
    Bool(bool),
    Path(PathBuf),
    List(Vec<Expression>),
    Variable(Variable),
    Call(FunctionCall),
    Interpolation(Vec<Expression>),
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub enum ResolvedExpression {
    String(String),
    Bool(bool),
    Path(PathBuf),
    ResolvedInterpolation(ResolvedInterpolation),
    List(Vec<ResolvedExpression>),
}

#[derive(Debug, Clone)]
pub struct ResolvedInterpolation(pub Vec<ResolvedInterpolationChunk>);

#[derive(Debug, Clone)]
pub enum ResolvedInterpolationChunk {
    String(String),
    Path(PathBuf),
}

#[derive(Debug, Clone)]
pub struct Variable(pub String);

#[derive(Clone, Copy)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[derive(Clone)]
pub struct Spanned<T> {
    pub item: T,
    pub span: Span,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.item, f)?;
        write!(f, " @ {:?}", self.span)
    }
}
