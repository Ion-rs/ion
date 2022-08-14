use crate::parser::ParserError;
use std::{error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IonError {
    MissingSection(Box<str>),
    MissingValue(Box<str>),
    ParserError(ParserError),
}

impl error::Error for IonError {
    fn description(&self) -> &str {
        "IonError"
    }
}

impl fmt::Display for IonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl From<ParserError> for IonError {
    fn from(error: ParserError) -> Self {
        Self::ParserError(error)
    }
}
