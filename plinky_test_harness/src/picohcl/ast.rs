#[derive(Debug, Clone)]
pub struct Document {
    pub contents: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Assignment(AssignmentStatement),
    Block(BlockStatement),
}

#[derive(Debug, Clone)]
pub struct AssignmentStatement {
    pub key: String,
    pub value: Expression,
}

#[derive(Debug, Clone)]
pub struct BlockStatement {
    pub kind: String,
    pub name: Option<String>,
    pub contents: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Expression {
    String(String),
    Bool(bool),
    List(Vec<Expression>),
    Variable(Variable),
    Interpolation(Interpolation),
}

#[derive(Debug, Clone)]
pub struct Interpolation(pub Vec<Expression>);

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
