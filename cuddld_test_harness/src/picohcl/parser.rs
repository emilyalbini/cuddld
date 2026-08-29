use crate::picohcl::ast::{
    AssignmentStatement, BlockStatement, Document, Expression, FunctionCall, Span, Spanned,
    Statement, Variable,
};
use crate::picohcl::lexer::{Lexer, LexerError, Token};
use cuddld_macros::{Display, Error};

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
        } else if let Token::OpenCurly | Token::OpenString = &peek.item {
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
        let name = if let Token::OpenString = self.lexer.peek()?.item {
            // We don't allow interpolations in block names, so we ensure it's a simple string.
            self.lexer.next()?;
            let string_token = self.lexer.next()?;
            let Token::RawString(name) = string_token.item else {
                return Err(ParseError::unexpected("non-interpolated string", string_token));
            };
            self.expect_token(Token::CloseString)?;
            Some(name)
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
        if let Token::OpenString = &peeked.item {
            Ok(self.parse_string_or_interpolation()?)
        } else if let Token::Literal(literal) = &peeked.item {
            if literal == "true" {
                self.lexer.next()?;
                Ok(Expression::Bool(true))
            } else if literal == "false" {
                self.lexer.next()?;
                Ok(Expression::Bool(false))
            } else {
                Ok(self.parse_variable()?)
            }
        } else if let Token::OpenSquare = &peeked.item {
            Ok(Expression::List(self.parse_list()?))
        } else {
            Err(ParseError::unexpected("expression", peeked))
        }
    }

    fn parse_variable(&mut self) -> Result<Expression, ParseError> {
        let mut variable = self.expect_literal()?;
        if let Token::OpenParen = self.lexer.peek()?.item {
            return self.parse_function(variable);
        }
        while let Token::Dot = self.lexer.peek()?.item {
            let _ = self.lexer.next()?;
            variable.push('.');
            variable.push_str(&self.expect_literal()?);
        }
        Ok(Expression::Variable(Variable(variable)))
    }

    fn parse_function(&mut self, name: String) -> Result<Expression, ParseError> {
        Ok(Expression::Call(FunctionCall {
            name,
            args: self.parse_comma_separated(Token::OpenParen, Token::CloseParen)?,
        }))
    }

    fn parse_list(&mut self) -> Result<Vec<Expression>, ParseError> {
        self.parse_comma_separated(Token::OpenSquare, Token::CloseSquare)
    }

    fn parse_comma_separated(
        &mut self,
        open: Token,
        close: Token,
    ) -> Result<Vec<Expression>, ParseError> {
        let mut result = Vec::new();
        self.expect_token(open)?;

        loop {
            if self.lexer.peek()?.item == close {
                self.lexer.next()?;
                return Ok(result);
            }
            result.push(self.parse_expression()?);

            let peeked = self.lexer.peek()?;
            if let Token::Comma = &peeked.item {
                self.lexer.next()?;
            } else if peeked.item == close {
                self.lexer.next()?;
                return Ok(result);
            } else {
                return Err(ParseError::unexpected("end or comma", peeked));
            }
        }
    }

    fn parse_string_or_interpolation(&mut self) -> Result<Expression, ParseError> {
        self.expect_token(Token::OpenString)?;

        let next = self.lexer.next()?;
        let mut interpolation = Vec::new();
        if let Token::RawString(string) = next.item {
            if let Token::CloseString = self.lexer.peek()?.item {
                self.expect_token(Token::CloseString)?;
                return Ok(Expression::String(string));
            } else {
                interpolation.push(Expression::String(string));
            }
        } else {
            return Err(ParseError::unexpected("string", next));
        }

        loop {
            if let Token::CloseString = self.lexer.peek()?.item {
                self.expect_token(Token::CloseString)?;
                return Ok(Expression::Interpolation(interpolation));
            }
            interpolation.push(self.parse_expression()?);

            let next = self.lexer.next()?;
            let Token::RawString(string) = next.item else {
                return Err(ParseError::unexpected("multiple exprs in interpolation", next));
            };
            interpolation.push(Expression::String(string));
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

    #[test]
    fn test_parse_interpolation() {
        assert_snapshot!(parse(
            r#"
            a = "hello"
            b = "hello ${user.name} world"
            c = "${foo}"
            d = "${"bar"}"
            e = "${"bar ${baz}"}"
            "#
        ))
    }

    #[test]
    fn test_function_call() {
        assert_snapshot!(parse(
            r#"
            a = foo()
            b = bar("a")
            c = baz("a", "b",)
            "#
        ))
    }
}
