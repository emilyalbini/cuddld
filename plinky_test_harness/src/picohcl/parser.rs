use super::ast::Variable;
use crate::picohcl::ast::{
    AssignmentStatement, BlockStatement, Document, Expression, Span, Spanned, Statement,
};
use crate::picohcl::lexer::{Lexer, LexerError, Token};
use plinky_macros::{Display, Error};

pub fn parse_picohcl(input: &str) -> Result<Document, ParseError> {
    let mut parser = Parser::new(input);
    parser.parse_document()
}

struct Parser<'a> {
    lexer: Lexer<'a>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { lexer: Lexer::new(input) }
    }
}

impl Parser<'_> {
    fn parse_document(&mut self) -> Result<Document, ParseError> {
        let mut contents = Vec::new();
        loop {
            self.skip_whitespace()?;
            if self.lexer.peek()?.item == Token::EndOfInput {
                break;
            }
            contents.push(self.parse_statement()?);
            if self.lexer.peek()?.item == Token::EndOfInput {
                break;
            }
            self.expect_token(Token::Newline)?;
        }

        Ok(Document { contents })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        let literal = self.expect_literal()?;
        let peek = self.lexer.peek()?;
        if let Token::Equal = &peek.item {
            Ok(Statement::Assignment(self.parse_assignment(literal)?))
        } else if let Token::OpenCurly | Token::String(_) = &peek.item {
            Ok(Statement::Block(self.parse_block(literal)?))
        } else {
            Err(ParseError::unexpected("statement", peek))
        }
    }

    fn parse_assignment(&mut self, key: String) -> Result<AssignmentStatement, ParseError> {
        self.expect_token(Token::Equal)?;
        let value = self.parse_expression()?;
        Ok(AssignmentStatement { key, value })
    }

    fn parse_block(&mut self, kind: String) -> Result<BlockStatement, ParseError> {
        let name = if let Token::String(_) = self.lexer.peek()?.item {
            Some(self.expect_string()?)
        } else {
            None
        };
        self.expect_token(Token::OpenCurly)?;

        let peek = self.lexer.peek()?.item;
        if let Token::Newline = &peek {
            let mut contents = Vec::new();
            loop {
                self.expect_token(Token::Newline)?;
                if let Token::CloseCurly = self.lexer.peek()?.item {
                    self.lexer.next()?;
                    break;
                }
                contents.push(self.parse_statement()?);
            }
            Ok(BlockStatement { kind, name, contents })
        } else if let Token::CloseCurly = &peek {
            self.lexer.next()?;
            Ok(BlockStatement { kind, name, contents: Vec::new() })
        } else {
            let statement = self.parse_statement()?;
            self.expect_token(Token::CloseCurly)?;

            Ok(BlockStatement { kind, name, contents: vec![statement] })
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        let peeked = self.lexer.peek()?;
        if let Token::String(_) = &peeked.item {
            Ok(Expression::String(self.expect_string()?))
        } else if let Token::Literal(literal) = &peeked.item {
            if literal == "true" {
                self.lexer.next()?;
                Ok(Expression::Bool(true))
            } else if literal == "false" {
                self.lexer.next()?;
                Ok(Expression::Bool(false))
            } else {
                Ok(Expression::Variable(self.parse_variable()?))
            }
        } else if let Token::OpenSquare = &peeked.item {
            Ok(Expression::List(self.parse_list()?))
        } else {
            Err(ParseError::unexpected("expression", peeked))
        }
    }

    fn parse_variable(&mut self) -> Result<Variable, ParseError> {
        let mut variable = self.expect_literal()?;
        while let Token::Dot = self.lexer.peek()?.item {
            let _ = self.lexer.next()?;
            variable.push('.');
            variable.push_str(&self.expect_literal()?);
        }
        Ok(Variable(variable))
    }

    fn parse_list(&mut self) -> Result<Vec<Expression>, ParseError> {
        let mut result = Vec::new();
        self.expect_token(Token::OpenSquare)?;

        loop {
            if let Token::CloseSquare = self.lexer.peek()?.item {
                self.lexer.next()?;
                return Ok(result);
            }
            result.push(self.parse_expression()?);

            let peeked = self.lexer.peek()?;
            if let Token::Comma = &peeked.item {
                self.lexer.next()?;
            } else if let Token::CloseSquare = &peeked.item {
                self.lexer.next()?;
                return Ok(result);
            } else {
                return Err(ParseError::unexpected("end of list or comma", peeked));
            }
        }
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        let next = self.lexer.next()?;
        if let Token::String(string) = next.item {
            Ok(string)
        } else {
            Err(ParseError::unexpected("string", next))
        }
    }

    fn expect_literal(&mut self) -> Result<String, ParseError> {
        let next = self.lexer.next()?;
        if let Token::Literal(lit) = next.item {
            Ok(lit)
        } else {
            Err(ParseError::unexpected("literal", next))
        }
    }

    fn expect_token(&mut self, token: Token) -> Result<(), ParseError> {
        let next = self.lexer.next()?;
        if token == next.item {
            Ok(())
        } else {
            Err(ParseError::unexpected(format!("{token:?}"), next))
        }
    }

    fn skip_whitespace(&mut self) -> Result<(), ParseError> {
        while let Ok(Spanned { item: Token::Newline, .. }) = self.lexer.peek() {
            self.lexer.next()?;
        }
        Ok(())
    }
}

#[derive(Debug, Error, Display)]
#[display("{span:?}: {kind}")]
pub struct ParseError {
    span: Span,
    kind: ParseErrorKind,
}

impl ParseError {
    fn unexpected(expected: impl Into<String>, token: Spanned<Token>) -> Self {
        Self { span: token.span, kind: ParseErrorKind::Unexpected(expected.into(), token.item) }
    }
}

#[derive(Debug, Display)]
enum ParseErrorKind {
    #[transparent]
    Lexer(LexerError),
    #[display("expected {f0}, found token: {f1:?}")]
    Unexpected(String, Token),
}

impl From<LexerError> for ParseError {
    fn from(lexer: LexerError) -> Self {
        Self { span: lexer.span, kind: ParseErrorKind::Lexer(lexer) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn parse(input: &str) -> String {
        format!("{:#?}", Parser::new(input).parse_document().expect("failed to parse input"))
    }

    #[track_caller]
    fn parse_error(input: &str) {
        dbg!(Parser::new(input).parse_document().expect_err("input parsed but shouldn't"));
    }

    #[test]
    fn test_expressions() {
        assert_snapshot!(parse(
            r#"
            a = "string"
            b = short-variable
            c = medium.variable
            d = long.variable.trust.me
            e = []
            f = ["a"]
            g = ["a", medium.variable]
            h = ["a",]
            i = true
            j = false
            "#
        ));
    }

    #[test]
    fn test_single_line_block() {
        assert_snapshot!(parse(
            r#"
            foo { bar = baz }
            foo "bar" { baz = quux }
            foo { bar { baz { quux = hello }}}
            "#
        ))
    }

    #[test]
    fn test_empty_block() {
        assert_snapshot!(parse(
            r#"
            foo {}
            foo "bar" { }
            foo {
            }
            foo "bar" {
            }
            "#
        ))
    }

    #[test]
    fn test_multiline_block() {
        assert_snapshot!(parse(
            r#"
            env {
                FOO = "1"
                BAR = "2"
            }
            env "sample" {
                enable {}
                FOO = "3"
            }
            "#
        ))
    }

    #[test]
    fn test_multiple_statements_without_newline() {
        parse_error("a = foo b = bar")
    }

    #[test]
    fn test_invalid_variable() {
        parse_error("a = foo..bar");
    }

    #[test]
    fn test_invalid_lists() {
        parse_error("a = [,]");
        parse_error("a = [foo bar]");
        parse_error("a = [foo,bar,,]");
    }
}
