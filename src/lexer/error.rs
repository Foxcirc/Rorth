
use crate::lexer::{constants::Pos, /* token::token::TokenKind */};
use std::fmt::{Display, Formatter, Error as FormatError};

/// An Error generated by the Lexer
#[derive(Debug, Clone)]
pub(crate) struct Error {
    pub(crate) kind: ErrorKind,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FormatError> {
        Ok(())
    }
}

impl std::error::Error for Error {

}
#[derive(Debug, Clone)]
pub(crate) enum ErrorKind {
    /// Invalid sequence while parsing Eg. an integer.
    InvalidSequence { pos: Pos /* [line, colum, char] */ },
    /// An invalid character wich doesn't match to any token.
    InvalidChar { chr: char, pos: Pos /* [line, colum, char] */ },
}
