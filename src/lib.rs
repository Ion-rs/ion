use std::collections::BTreeMap;

#[macro_use]
mod ion;
mod parser;
mod writer;

pub use self::ion::{FromIon, Ion, IonError, Section, Value};
pub use self::parser::{Parser, ParserError};
pub use self::writer::Writer;

pub type Dictionary = BTreeMap<String, Value>;
pub type Row = Vec<Value>;
