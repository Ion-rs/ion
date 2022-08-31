mod display;
mod from_ion;
mod from_row;
mod ion_error;
#[macro_use]
mod macros;
mod section;
mod value;

use crate::Parser;
use std::collections::BTreeMap;
use std::str;

pub use self::from_ion::FromIon;
pub use self::from_row::FromRow;
pub use self::ion_error::IonError;
pub use self::section::Section;
pub use self::value::Value;

pub type Dictionary = BTreeMap<String, Value>;
pub type Row = Vec<Value>;

#[cfg(feature = "serde-json")]
use serde::Serialize;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde-json", derive(Serialize))]
pub struct Ion {
    #[cfg_attr(feature = "serde-json", serde(flatten))]
    sections: BTreeMap<String, Section>,
}

impl Ion {
    pub fn new(map: BTreeMap<String, Section>) -> Ion {
        Ion { sections: map }
    }

    pub fn from_str_filtered(text: &str, accepted_sections: Vec<&str>) -> Result<Self, IonError> {
        parser_to_ion(Parser::new_filtered(text, accepted_sections))
    }

    pub fn get<K>(&self, key: K) -> Option<&Section>
    where
        K: AsRef<str>,
    {
        self.sections.get(key.as_ref())
    }

    pub fn fetch<K>(&self, key: K) -> Result<&Section, IonError>
    where
        K: AsRef<str>,
    {
        self.get(key.as_ref())
            .ok_or_else(|| IonError::MissingSection(key.as_ref().into()))
    }

    /// Removes a `Section` from the ion structure and returning it
    pub fn remove<K>(&mut self, key: K) -> Option<Section>
    where
        K: AsRef<str>,
    {
        self.sections.remove(key.as_ref())
    }

    pub fn iter(&self) -> ::std::collections::btree_map::Iter<String, Section> {
        self.sections.iter()
    }
}

impl str::FromStr for Ion {
    type Err = IonError;

    fn from_str(text: &str) -> Result<Self, IonError> {
        parser_to_ion(Parser::new(text))
    }
}

fn parser_to_ion(mut parser: Parser) -> Result<Ion, IonError> {
    parser.read().map(Ion::new).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_string() {
        let v = Value::String("foo".into());
        assert_eq!(Some("foo"), v.as_str());
        let v = Value::Integer(1);
        assert_eq!(None, v.as_str());
    }

    #[test]
    fn as_boolean() {
        let v = Value::Boolean(true);
        assert_eq!(Some(true), v.as_boolean());
        let v = Value::Integer(1);
        assert_eq!(None, v.as_boolean());
    }

    #[test]
    fn as_integer() {
        let v = Value::Integer(1);
        assert_eq!(Some(1), v.as_integer());
        let v = Value::String("foo".into());
        assert_eq!(None, v.as_integer());
    }

    #[test]
    fn as_str() {
        let v = Value::String("foo".into());
        assert_eq!(Some("foo"), v.as_str());
        let v = Value::Integer(1);
        assert_eq!(None, v.as_str());
    }

    #[test]
    fn row_without_header() {
        let ion = ion!(
            r#"
            [FOO]
            |1||2|
            |1|   |2|
            |1|2|3|
        "#
        );

        let rows = ion.get("FOO").unwrap().rows_without_header();
        assert!(rows.len() == 3);
    }

    #[test]
    fn row_with_header() {
        let ion = ion!(
            r#"
            [FOO]
            | 1 | 2 | 3 |
            |---|---|---|
            |1||2|
            |1|   |2|
        "#
        );

        let rows = ion.get("FOO").unwrap().rows_without_header();
        assert!(rows.len() == 2);
    }

    #[test]
    fn no_rows_with_header() {
        let ion = ion!(
            r#"
            [FOO]
            | 1 | 2 | 3 |
            |---|---|---|
        "#
        );

        let rows = ion.get("FOO").unwrap().rows_without_header();
        assert_eq!(0, rows.len());
    }

    #[test]
    fn filtered_section() {
        let ion = ion_filtered!(
            r#"
            [FOO]
            |1||2|
            |1|   |2|
            |1|2|3|
            [BAR]
            |1||2|
        "#,
            vec!["FOO"]
        );

        let rows = ion.get("FOO").unwrap().rows_without_header();
        assert_eq!(3, rows.len());
        assert!(ion.get("BAR").is_none());
    }
}
