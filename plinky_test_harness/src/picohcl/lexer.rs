use crate::picohcl::ast::{Span, Spanned};
use plinky_macros::{Display, Error};
use std::collections::VecDeque;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Token {
    Literal(String),
    String(String),
    OpenSquare,
    CloseSquare,
    OpenCurly,
    CloseCurly,
    Equal,
    Dot,
    Comma,
    Newline,
    EndOfInput,
}

pub(super) struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
    span: Span,
    next_span: Span,
    queued_tokens: VecDeque<Result<Spanned<Token>, LexerError>>,
}

impl<'a> Lexer<'a> {
    pub(super) fn new(input: &'a str) -> Self {
        Self {
            input: input.chars().peekable(),
            span: Span { line: 0, col: 0 },
            next_span: Span { line: 0, col: 0 },
            queued_tokens: VecDeque::new(),
        }
    }
}

impl Lexer<'_> {
    pub(super) fn next(&mut self) -> Result<Spanned<Token>, LexerError> {
        if let Some(queued) = self.queued_tokens.pop_front() {
            return queued;
        }
        'lexer: loop {
            match self.next_char() {
                None => return Ok(self.spanned(Token::EndOfInput)),

                // Newline
                Some('\n') => return Ok(self.spanned(Token::Newline)),

                // Symbols
                Some('{') => return Ok(self.spanned(Token::OpenCurly)),
                Some('}') => return Ok(self.spanned(Token::CloseCurly)),
                Some('[') => return Ok(self.spanned(Token::OpenSquare)),
                Some(']') => return Ok(self.spanned(Token::CloseSquare)),
                Some('=') => return Ok(self.spanned(Token::Equal)),
                Some('.') => return Ok(self.spanned(Token::Dot)),
                Some(',') => return Ok(self.spanned(Token::Comma)),

                // Literals
                Some(c) if is_literal_start(c) => {
                    let mut literal = String::from(c);
                    while let Some(n) = self.peek_char()
                        && is_literal_continue(n)
                    {
                        self.next_char();
                        literal.push(n);
                    }
                    return Ok(self.spanned(Token::Literal(literal)));
                }

                // Strings
                Some('"') => {
                    let mut string = String::new();
                    let mut escape = false;
                    loop {
                        let Some(chr) = self.next_char() else {
                            return Err(self.err(LexerErrorKind::UnterminatedString));
                        };
                        if escape {
                            escape = false;
                            match chr {
                                'n' => string.push('\n'),
                                't' => string.push('\t'),
                                '"' => string.push('"'),
                                '\\' => string.push('\\'),
                                _ => return Err(self.err(LexerErrorKind::InvalidEscape(chr))),
                            }
                        } else if chr == '\\' {
                            escape = true;
                        } else if chr == '"' {
                            return Ok(self.spanned(Token::String(string)));
                        } else {
                            string.push(chr);
                        }
                    }
                }

                // Whitespace
                Some(' ' | '\r' | '\t') => {}

                // Block comment
                Some('/') if self.peek_char() == Some('*') => {
                    self.next_char();
                    loop {
                        if self.peek_char().is_none() {
                            return Err(self.err(LexerErrorKind::UnterminatedComment));
                        }
                        if self.next_char() == Some('*') {
                            if self.next_char() == Some('/') {
                                continue 'lexer;
                            }
                        }
                    }
                }

                // Line comment
                Some('/') if self.peek_char() == Some('/') => {
                    self.next_char(); // Consume the /.
                    loop {
                        let next = self.next_char();
                        if next == Some('\n') {
                            return Ok(self.spanned(Token::Newline));
                        } else if next == None {
                            return Ok(self.spanned(Token::EndOfInput));
                        }
                    }
                }

                Some(chr) => return Err(self.err(LexerErrorKind::InvalidChar(chr))),
            }
        }
    }

    pub(super) fn peek(&mut self) -> Result<Spanned<Token>, LexerError> {
        if self.queued_tokens.len() == 0 {
            let token = self.next();
            self.queued_tokens.push_back(token);
        }
        self.queued_tokens.front().unwrap().clone()
    }

    fn next_char(&mut self) -> Option<char> {
        self.span = self.next_span;
        match self.input.next() {
            Some('\n') => {
                self.next_span.line += 1;
                self.next_span.col = 0;
                Some('\n')
            }
            Some(c) => {
                self.next_span.col += 1;
                Some(c)
            }
            None => None,
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.input.peek().copied()
    }

    fn err(&self, kind: LexerErrorKind) -> LexerError {
        LexerError { span: self.span, kind }
    }

    fn spanned<T>(&self, item: T) -> Spanned<T> {
        Spanned { item, span: self.span }
    }
}

#[derive(Debug, Error, Display, Clone)]
#[display("{span:?}: {kind}")]
pub(super) struct LexerError {
    pub(super) span: Span,
    pub(super) kind: LexerErrorKind,
}

#[derive(Debug, Display, Clone)]
pub(super) enum LexerErrorKind {
    #[display("unterminated string")]
    UnterminatedString,
    #[display("unterminated comment")]
    UnterminatedComment,
    #[display("invalid char: {f0:?}")]
    InvalidChar(char),
    #[display("invalid escape: \\{f0}")]
    InvalidEscape(char),
}

fn is_literal_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_literal_continue(c: char) -> bool {
    is_literal_start(c) || c.is_ascii_digit() || c == '-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn lex(input: &str) -> String {
        let mut lexer = Lexer::new(input);
        let mut result = Vec::new();
        loop {
            match lexer.next() {
                Ok(token @ Spanned { item: Token::EndOfInput, .. }) => {
                    result.push(token);
                    break;
                }
                Ok(token) => result.push(token),
                Err(err) => panic!("lexing failed: {err}"),
            }
        }
        format!("{result:#?}")
    }

    #[test]
    fn test_lexer() {
        let test = "hello =    /* [] */ . // hello \n[\"world\\n\\\\\", foo.bar}{]\nBAZ";
        assert_snapshot!(lex(test));
    }

    #[test]
    fn test_lex_string_interpolation() {
        let test = "hello = \"world ${foo.bar}\"";
        assert_snapshot!(lex(test));
    }
}
