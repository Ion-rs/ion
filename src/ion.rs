mod display;
mod from_ion;
mod from_row;
mod ion_error;
mod section;
mod value;

pub use self::from_ion::*;
pub use self::from_row::*;
pub use self::ion_error::*;
pub use self::section::*;
pub use self::value::*;
use crate::Parser;
use std::collections::BTreeMap;
use std::str;

#[derive(Debug)]
pub struct Ion {
    sections: BTreeMap<String, Section>,
}

impl Ion {
    pub fn new(sections: BTreeMap<String, Section>) -> Ion {
        Ion { sections }
    }

    pub fn from_str_filtered(s: &str, accepted_sections: Vec<&str>) -> Result<Self, IonError> {
        parser_to_ion(Parser::new_filtered(s, accepted_sections))
    }

    pub fn get(&self, key: &str) -> Option<&Section> {
        self.sections.get(key)
    }

    pub fn fetch(&self, key: &str) -> Result<&Section, IonError> {
        self.get(key)
            .ok_or_else(|| IonError::MissingSection(key.to_owned()))
    }

    pub fn remove(&mut self, key: &str) -> Option<Section> {
        self.sections.remove(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Section)> {
        self.sections.iter()
    }
}

impl str::FromStr for Ion {
    type Err = IonError;

    fn from_str(s: &str) -> Result<Ion, IonError> {
        parser_to_ion(Parser::new(s))
    }
}

fn parser_to_ion(mut parser: Parser) -> Result<Ion, IonError> {
    match parser.read() {
        Some(ion) => Ok(Ion::new(ion)),
        None => Err(IonError::ParserErrors(parser.errors)),
    }
}

#[macro_export]
macro_rules! ion {
    ($raw:expr) => {{ $raw.parse::<Ion>().expect("Failed parsing to 'Ion'") }};
}

#[macro_export]
macro_rules! ion_filtered {
    ($raw:expr, $accepted_sections:expr) => {
        Ion::from_str_filtered($raw, $accepted_sections)
            .expect("Failed parsing by 'from_str_filtered' to 'Ion'")
    };
}

#[cfg(test)]
mod tests {
    use crate::{Ion, Value};

    #[test]
    fn as_string() {
        let v = Value::String("foo".into());
        assert_eq!(Some(&"foo".into()), v.as_string());

        let v = Value::Integer(1);
        assert_eq!(None, v.as_string());
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
