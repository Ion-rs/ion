use crate::{Section, Value};
use std::collections::BTreeMap;
use std::iter::Peekable;
use std::{error, fmt, str};

type ParseResultOpt<T> = Result<Option<T>, ParserError>;
type ParseResult<T> = Result<T, ParserError>;

#[derive(Debug, PartialEq)]
pub enum Element {
    Section(String),
    Row(Vec<Value>),
    Entry(String, Value),
    Comment(String),
}

pub struct Parser<'a> {
    input: &'a str,
    cur: Peekable<str::CharIndices<'a>>,
    accepted_sections: Option<Vec<&'a str>>,
    section_capacity: usize,
    row_capacity: usize,
    array_capacity: usize,
    last_section: Option<Box<str>>,
}

impl<'a> Iterator for Parser<'a> {
    type Item = Result<Element, ParserError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut is_section_accepted = true;
        loop {
            self.ws();
            if self.newline() {
                continue;
            }

            let c = match self.cur.peek() {
                Some((_, c)) => *c,
                None => return None,
            };

            if c == '[' {
                let section_name = self.section_name();
                match self.is_section_accepted(&section_name) {
                    Some(true) => return Some(Ok(Element::Section(section_name))),
                    Some(false) => is_section_accepted = false,
                    None => return None,
                };
            }
            if !is_section_accepted {
                self.skip_line();
                continue;
            }

            return match c {
                '|' => self.row().map(Ok),
                '#' => self.comment().map(Ok),
                _ => self.entry().transpose(),
            };
        }
    }
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Parser<'a> {
        Self::new_filtered_opt(input, None)
    }

    pub fn new_filtered(input: &'a str, accepted_sections: Vec<&'a str>) -> Parser<'a> {
        Self::new_filtered_opt(input, Some(accepted_sections))
    }

    pub fn with_section_capacity(mut self, section_capacity: usize) -> Self {
        self.section_capacity = section_capacity;
        self
    }

    pub fn with_row_capacity(mut self, row_capacity: usize) -> Self {
        self.row_capacity = row_capacity;
        self
    }

    pub fn with_array_capacity(mut self, array_capacity: usize) -> Self {
        self.array_capacity = array_capacity;
        self
    }

    fn new_filtered_opt(input: &'a str, accepted_sections: Option<Vec<&'a str>>) -> Parser<'a> {
        Parser {
            input,
            cur: input.char_indices().peekable(),
            accepted_sections,
            section_capacity: 16,
            row_capacity: 8,
            array_capacity: 2,
            last_section: None,
        }
    }

    pub fn read(&mut self) -> ParseResult<BTreeMap<String, Section>> {
        let mut map = BTreeMap::new();

        let mut cur_section = Section::with_capacity(self.section_capacity);
        let mut last_name = None;

        while let Some(element) = self.next().transpose()? {
            match element {
                Element::Section(name) => {
                    if let Some(last_name) = last_name {
                        map.insert(last_name, cur_section);
                    }
                    last_name = Some(name);
                    cur_section = Section::with_capacity(self.section_capacity);
                }
                Element::Row(row) => cur_section.rows.push(row),
                Element::Entry(key, value) => {
                    cur_section.dictionary.insert(key, value);
                }
                _ => continue,
            };
        }

        match last_name {
            Some(name) => map.insert(name, cur_section),
            None if self.accepted_sections.is_none() => {
                map.insert(Section::DEFAULT_NAME.to_string(), cur_section)
            }
            _ => None,
        };

        Ok(map)
    }

    /// Peeks and checks if the next chars are like `'\t'` or `' '` then read them.
    /// Stops after first other char
    fn ws(&mut self) {
        while let Some((_, '\t')) | Some((_, ' ')) = self.cur.peek() {
            self.cur.next();
        }
    }

    fn newline(&mut self) -> bool {
        match self.cur.peek() {
            Some((_, '\n')) => {
                self.cur.next();
                true
            }

            Some((_, '\r')) => {
                self.cur.next();

                if let Some((_, '\n')) = self.cur.peek() {
                    self.cur.next();
                }

                true
            }

            _ => false,
        }
    }

    #[allow(clippy::skip_while_next)]
    fn skip_line(&mut self) {
        // suggested by clippy change `self.cur.by_ref().find(|(_, c)| *c != '\n');` slows down the parser in some cases twice
        self.cur.by_ref().skip_while(|&(_, c)| c != '\n').next();
    }

    fn comment(&mut self) -> Option<Element> {
        if !self.eat('#') {
            return None;
        }

        Some(Element::Comment(
            self.slice_to_inc('\n').unwrap_or("").into(),
        ))
    }

    fn eat(&mut self, ch: char) -> bool {
        match self.cur.peek() {
            Some((_, c)) if *c == ch => {
                self.cur.next();
                true
            }
            _ => false,
        }
    }

    fn section_name(&mut self) -> String {
        self.eat('[');
        self.ws();

        let retval = self
            .cur
            .by_ref()
            .map(|(_, c)| c)
            .take_while(|c| *c != ']')
            .collect::<String>();
        self.last_section = Some(retval.clone().into());
        retval
    }

    fn value(&mut self) -> ParseResultOpt<Value> {
        self.ws();
        self.newline();
        self.ws();

        match self.cur.peek() {
            Some((_, '"')) => self.finish_string(),
            Some((_, '[')) => self.finish_array(),
            Some((_, '{')) => self.finish_dictionary(),
            Some((_, ch)) if is_digit(*ch) => self.number(),
            Some((pos, 't')) | Some((pos, 'f')) => {
                let pos = *pos;
                self.boolean(pos)
            }
            _ => Err(self.create_error("Cannot read a value")),
        }
    }

    fn finish_string(&mut self) -> ParseResultOpt<Value> {
        self.cur.next();

        self.slice_to_exc('"')
            .map(|v| Value::String(v.to_string()))
            .ok_or_else(|| self.create_error("Cannot finish string"))
            .map(Some)
    }

    fn finish_array(&mut self) -> ParseResultOpt<Value> {
        self.cur.next();
        let mut row = Vec::with_capacity(self.array_capacity);

        loop {
            self.ws();
            if let Some((_, ch)) = self.cur.peek() {
                match ch {
                    ']' => {
                        self.cur.next();
                        return Ok(Some(Value::Array(row)));
                    }
                    ',' => {
                        self.cur.next();
                        continue;
                    }
                    _ => match self.value()? {
                        Some(v) => row.push(v),
                        None => break,
                    },
                }
            } else {
                break;
            }
        }

        Err(self.create_error("Cannot finish an array"))
    }

    fn finish_dictionary(&mut self) -> ParseResultOpt<Value> {
        self.cur.next();
        let mut map = BTreeMap::new();

        loop {
            self.ws();
            if let Some((_, ch)) = self.cur.peek() {
                match ch {
                    '}' => {
                        self.cur.next();
                        return Ok(Some(Value::Dictionary(map)));
                    }
                    ',' => {
                        self.cur.next();
                        continue;
                    }
                    '\n' => {
                        self.cur.next();
                        continue;
                    }
                    _ => {
                        if let Some(Element::Entry(k, v)) = self.entry()? {
                            map.insert(k, v);
                        } else {
                            return Err(self.create_error("Wrong entry of a dictionary"));
                        }
                    }
                }
            } else {
                break;
            }
        }

        Err(self.create_error("Cannot finish a dictionary"))
    }

    fn entry(&mut self) -> ParseResultOpt<Element> {
        let key = if let Some(key) = self.key_name() {
            key
        } else {
            return Ok(None);
        };

        if !self.keyval_sep() {
            return Err(self.create_error("Expected the '=' key value separator"));
        }

        self.value()
            .map(|val| val.map(|v| Element::Entry(key.to_string(), v)))
    }

    fn key_name(&mut self) -> Option<&'a str> {
        self.slice_while(|ch| matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-'))
    }

    fn keyval_sep(&mut self) -> bool {
        self.ws();
        if !self.expect('=') {
            return false;
        }
        self.ws();
        true
    }

    fn number(&mut self) -> ParseResultOpt<Value> {
        let mut is_float = false;
        let prefix = if let Some(prefix) = self.integer() {
            prefix
        } else {
            return Ok(None);
        };
        let decimal = if self.eat('.') {
            is_float = true;
            self.integer()
        } else {
            None
        };

        if is_float {
            format!("{}.{}", prefix, decimal.unwrap_or(""))
                .parse::<f64>()
                .map(Into::into)
                .map(Some)
                .map_err(|e| self.create_error(e.to_string()))
        } else {
            prefix
                .parse::<i64>()
                .map(Into::into)
                .map(Some)
                .map_err(|e| self.create_error(e.to_string()))
        }
    }

    fn integer(&mut self) -> Option<&'a str> {
        self.slice_while(|ch| matches!(ch, '0'..='9'))
    }

    fn boolean(&mut self, start: usize) -> ParseResultOpt<Value> {
        let rest = &self.input[start..];

        if rest.starts_with("true") {
            for _ in 0..4 {
                self.cur.next();
            }
            Ok(Some(Value::Boolean(true)))
        } else if rest.starts_with("false") {
            for _ in 0..5 {
                self.cur.next();
            }
            Ok(Some(Value::Boolean(false)))
        } else {
            Ok(None)
        }
    }

    fn expect(&mut self, ch: char) -> bool {
        self.eat(ch)
    }

    fn row(&mut self) -> Option<Element> {
        let mut row = Vec::with_capacity(self.row_capacity);
        self.eat('|');

        loop {
            self.ws();
            if self.comment().is_some() {
                break;
            } // this will eat and NOT return comments within tables
            if self.newline() {
                break;
            }
            if self.cur.peek().is_none() {
                break;
            }

            row.push(Value::String(self.cell()));
        }

        Some(Element::Row(row))
    }

    fn cell(&mut self) -> String {
        self.ws();
        self.slice_to_exc('|')
            .map(str::trim_end)
            .unwrap_or("")
            .to_owned()
    }

    fn is_section_accepted(&mut self, name: &str) -> Option<bool> {
        let sections = match self.accepted_sections {
            Some(ref mut sections) => sections,
            None => return Some(true),
        };
        if sections.is_empty() {
            return None;
        }
        match sections.iter().position(|s| *s == name) {
            Some(idx) => {
                sections.swap_remove(idx);
                Some(true)
            }
            None => Some(false),
        }
    }

    // returns slice from the next character to `ch`, inclusive
    // after this function, self.cur.next() returns the next character after `ch`
    // None is only returned if the input is empty
    // Examples:
    // Parser::new("foObar").slice_to_inc('b') == Some("foOb"), self.cur.next() == (4, 'a')
    // Parser::new("foObar").slice_to_inc('f') == Some("f"),    self.cur.next() == (1, 'o')
    fn slice_to_inc(&mut self, ch: char) -> Option<&'a str> {
        self.cur.next().map(|(start, c)| {
            if c == ch {
                &self.input[start..=start]
            } else {
                self.cur
                    .find(|(_, c)| *c == ch)
                    .map_or(&self.input[start..], |(end, _)| &self.input[start..=end])
            }
        })
    }

    // returns slice from the next character to `ch`
    // the result is exclusive (does not contain `ch`), but the functions consumes `ch`
    // None is returned when the input is empty or when `ch` is the next character
    // Examples:
    // Parser::new("foObar").slice_to_exc('b') == Some("foO"), self.cur.next() == (4, 'a')
    // Parser::new("foObar").slice_to_exc('f') == None,        self.cur.next() == (1, 'o')
    fn slice_to_exc(&mut self, ch: char) -> Option<&'a str> {
        self.cur.next().map(|(start, c)| {
            if c == ch {
                ""
            } else {
                self.cur
                    .find(|(_, c)| *c == ch)
                    .map_or(&self.input[start..], |(end, _)| &self.input[start..end])
            }
        })
    }

    // returns slice from the next character to the last consecutive character matching the predicate
    // the result is exclusive (does not contain `ch`) and does not consume `ch`
    // None is returned when the input is empty or when `ch` is the next character
    // Examples:
    // Parser::new("foObar").slice_while(|c| c != 'b') == Some("foO"), self.cur.next() == (3, 'b')
    // Parser::new("foObar").slice_while(|c| c != 'f') == None,        self.cur.next() == (0, 'f')
    fn slice_while(&mut self, predicate: impl Fn(char) -> bool) -> Option<&'a str> {
        self.cur.peek().cloned().and_then(|(start, c)| {
            if !predicate(c) {
                None
            } else {
                self.cur.next();

                while let Some(&(end, c)) = self.cur.peek() {
                    if !predicate(c) {
                        return Some(&self.input[start..end]);
                    }

                    self.cur.next();
                }

                Some(&self.input[start..])
            }
        })
    }

    fn create_error<M>(&mut self, message: M) -> ParserError
    where
        M: Into<Box<str>>,
    {
        ParserError {
            section: self
                .last_section
                .clone()
                .unwrap_or_else(|| "unknown".into()),
            desc: message.into(),
        }
    }
}

fn is_digit(c: char) -> bool {
    matches!(c, '0'..='9')
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserError {
    pub section: Box<str>,
    pub desc: Box<str>,
}

impl error::Error for ParserError {
    fn description(&self) -> &str {
        "error parsing Ion"
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Dictionary, Parser, Section, Value};
    use std::collections::BTreeMap;

    macro_rules! ext_ok_some {
        ($target:expr) => {
            $target
                .expect("Expected Ok got Err")
                .expect("Expected Some got None")
        };
    }

    macro_rules! ext_err {
        ($target:expr) => {
            $target.expect_err("Expected Err got Ok")
        };
    }

    macro_rules! target {
        ($text:expr) => {
            Parser::new($text)
        };
    }

    mod finish_string {
        use super::*;

        #[test]
        fn ok() {
            let mut target = target!("\"foObar\"");
            assert_eq!(
                Some("foObar"),
                ext_ok_some!(target.finish_string()).as_str()
            );

            let mut target = target!("\"foObar");
            assert_eq!(
                Some("foObar"),
                ext_ok_some!(target.finish_string()).as_str()
            );

            let mut target = target!("\"\"");
            assert_eq!(Some(""), ext_ok_some!(target.finish_string()).as_str());
        }

        #[test]
        fn err() {
            let mut target = target!("\"");
            assert_eq!(
                "ParserError { section: \"unknown\", desc: \"Cannot finish string\" }",
                ext_err!(target.finish_string()).to_string()
            );

            let mut target = target!("");
            assert_eq!(
                "ParserError { section: \"unknown\", desc: \"Cannot finish string\" }",
                ext_err!(target.finish_string()).to_string()
            );
        }
    }

    mod finish_array {
        use super::*;

        #[test]
        fn ok() {
            let mut target = target!("[]");
            assert_eq!(Value::Array(vec![]), ext_ok_some!(target.finish_array()));

            let mut target = target!("[\"a\", 4, 5.6]");
            assert_eq!(
                Value::Array(vec!["a".into(), 4.into(), 5.6.into()]),
                ext_ok_some!(target.finish_array())
            );
        }

        #[test]
        fn err() {
            let mut target = target!("[\"a\"");
            assert_eq!(
                "ParserError { section: \"unknown\", desc: \"Cannot finish an array\" }",
                ext_err!(target.finish_array()).to_string()
            );

            let mut target = target!("[");
            assert_eq!(
                "ParserError { section: \"unknown\", desc: \"Cannot finish an array\" }",
                ext_err!(target.finish_array()).to_string()
            );
        }
    }

    mod finish_dictionary {
        use super::*;

        #[test]
        fn ok() {
            let mut target = target!("{}");
            assert_eq!(
                Value::Dictionary(Dictionary::new()),
                ext_ok_some!(target.finish_dictionary())
            );

            let mut target = target!("{ foo = [\"bar\"] }");
            assert_eq!(
                "{ foo = [ \"bar\" ] }",
                ext_ok_some!(target.finish_dictionary()).to_string()
            );
        }

        #[test]
        fn err() {
            let mut target = target!("{");
            assert_eq!(
                "ParserError { section: \"unknown\", desc: \"Cannot finish a dictionary\" }",
                ext_err!(target.finish_dictionary()).to_string()
            );

            let mut target = target!("{ foo");
            assert_eq!(
                "ParserError { section: \"unknown\", desc: \"Expected the '=' key value separator\" }",
                ext_err!(target.finish_dictionary()).to_string()
            );

            let mut target = target!("{ foo = ");
            assert_eq!(
                "ParserError { section: \"unknown\", desc: \"Cannot read a value\" }",
                ext_err!(target.finish_dictionary()).to_string()
            );

            let mut target = target!("{ foo = \"bar\"");
            assert_eq!(
                "ParserError { section: \"unknown\", desc: \"Cannot finish a dictionary\" }",
                ext_err!(target.finish_dictionary()).to_string()
            );

            let mut target = target!("{ foo = [\"bar\"");
            assert_eq!(
                "ParserError { section: \"unknown\", desc: \"Cannot finish an array\" }",
                ext_err!(target.finish_array()).to_string()
            );

            let mut target = target!("{ foo = [\"bar\"]");
            assert_eq!(
                "ParserError { section: \"unknown\", desc: \"Cannot finish a dictionary\" }",
                ext_err!(target.finish_dictionary()).to_string()
            );

            let mut target = target!("{ | foo |");
            assert_eq!(
                "ParserError { section: \"unknown\", desc: \"Wrong entry of a dictionary\" }",
                ext_err!(target.finish_dictionary()).to_string()
            );
        }
    }

    #[test]
    fn slice_to_inc() {
        let mut target = target!("foObar");
        assert_eq!(Some("foOb"), target.slice_to_inc('b'));
        assert_eq!(Some((4, 'a')), target.cur.next());

        let mut target = target!("foObar");
        assert_eq!(Some("f"), target.slice_to_inc('f'));
        assert_eq!(Some((1, 'o')), target.cur.next());
    }

    #[test]
    fn slice_to_exc() {
        let mut target = target!("foObar");
        assert_eq!(Some("foO"), target.slice_to_exc('b'));
        assert_eq!(Some((4, 'a')), target.cur.next());

        let mut target = target!("foObar");
        assert_eq!(Some(""), target.slice_to_exc('f'));
        assert_eq!(Some((1, 'o')), target.cur.next());
    }

    #[test]
    fn slice_while() {
        let mut target = target!("foObar");
        assert_eq!(Some("foO"), target.slice_while(|c| c != 'b'));
        assert_eq!(Some((3, 'b')), target.cur.next());

        let mut target = target!("foObar");
        assert_eq!(None, target.slice_while(|c| c != 'f'));
        assert_eq!(Some((0, 'f')), target.cur.next());
    }

    mod iterator {
        use super::*;

        macro_rules! next_some_ok {
            ($target:expr) => {
                $target
                    .next()
                    .expect("Expected Some got None")
                    .expect("Expected Ok got Err")
            };
        }

        #[test]
        fn next_returns() {
            let raw = r#"
                [dict]
                first = "first"
                # comment
                second ="another"
                whitespace = "  "
                empty = ""
                some_bool = true

                ary = [ "col1", 2,"col3", false]

                [table]

                |abc|def|
                |---|---|
                |one|two|
                # comment
                |  1| 2 |
                |  2| 3 |

                [three]
                a=1
                B=2
                | this |
            "#;

            let mut target = target!(raw);

            assert_eq!(Element::Section("dict".to_owned()), next_some_ok!(target));
            assert_eq!(
                Element::Entry("first".to_owned(), Value::String("first".to_owned())),
                next_some_ok!(target)
            );
            assert_eq!(Element::Comment(" comment\n".into()), next_some_ok!(target));
            assert_eq!(
                Element::Entry("second".to_owned(), Value::String("another".to_owned())),
                next_some_ok!(target)
            );
            assert_eq!(
                Element::Entry("whitespace".to_owned(), Value::String("  ".to_owned())),
                next_some_ok!(target)
            );
            assert_eq!(
                Element::Entry("empty".to_owned(), Value::String("".to_owned())),
                next_some_ok!(target)
            );
            assert_eq!(
                Element::Entry("some_bool".to_owned(), Value::Boolean(true)),
                next_some_ok!(target)
            );
            assert_eq!(
                Element::Entry(
                    "ary".to_owned(),
                    Value::Array(vec![
                        Value::String("col1".to_owned()),
                        Value::Integer(2),
                        Value::String("col3".to_owned()),
                        Value::Boolean(false)
                    ])
                ),
                next_some_ok!(target)
            );

            assert_eq!(Element::Section("table".to_owned()), next_some_ok!(target));
            assert_eq!(
                Element::Row(vec![
                    Value::String("abc".to_owned()),
                    Value::String("def".to_owned())
                ]),
                next_some_ok!(target)
            );
            assert_eq!(
                Element::Row(vec![
                    Value::String("---".to_owned()),
                    Value::String("---".to_owned())
                ]),
                next_some_ok!(target)
            );
            assert_eq!(
                Element::Row(vec![
                    Value::String("one".to_owned()),
                    Value::String("two".to_owned())
                ]),
                next_some_ok!(target)
            );
            assert_eq!(Element::Comment(" comment\n".into()), next_some_ok!(target));
            assert_eq!(
                Element::Row(vec![
                    Value::String("1".to_owned()),
                    Value::String("2".to_owned())
                ]),
                next_some_ok!(target)
            );
            assert_eq!(
                Element::Row(vec![
                    Value::String("2".to_owned()),
                    Value::String("3".to_owned())
                ]),
                next_some_ok!(target)
            );
            assert_eq!(Element::Section("three".to_owned()), next_some_ok!(target));
            assert_eq!(
                Element::Entry("a".to_owned(), Value::Integer(1)),
                next_some_ok!(target)
            );
            assert_eq!(
                Element::Entry("B".to_owned(), Value::Integer(2)),
                next_some_ok!(target)
            );
            assert_eq!(
                Element::Row(vec![Value::String("this".to_owned())]),
                next_some_ok!(target)
            );
            assert_eq!(None, target.next());
            assert_eq!(None, target.next());
        }
    }

    mod read {
        use super::*;

        mod when_parsing_without_filtering {
            use super::*;

            mod and_ion_has_root_section {
                use super::*;

                mod and_root_section_has_dictionary_with_string {
                    use super::*;

                    #[test]
                    fn then_returns_dictionary() {
                        let raw = r#"
                            foo = "bar"
                        "#;
                        let mut target = target!(raw);

                        let actual = target.read().expect("Read failed");

                        let mut expected = BTreeMap::new();
                        let mut section = Section::new();
                        section
                            .dictionary
                            .insert("foo".to_owned(), Value::String("bar".to_owned()));
                        expected.insert("root".to_owned(), section);
                        assert_eq!(expected, actual);
                    }
                }

                mod and_root_section_has_dictionary_with_array {
                    use super::*;

                    #[test]
                    fn then_returns_dictionary() {
                        let raw = r#"
                            arr = ["WAW", "WRO"]
                        "#;
                        let mut target = target!(raw);

                        let actual = target.read().expect("Read failed");

                        let mut expected = BTreeMap::new();
                        let mut section = Section::new();
                        let array = vec![
                            Value::String("WAW".to_owned()),
                            Value::String("WRO".to_owned()),
                        ];
                        section
                            .dictionary
                            .insert("arr".to_owned(), Value::Array(array));
                        expected.insert("root".to_owned(), section);
                        assert_eq!(expected, actual);
                    }
                }

                mod and_root_section_has_dictionary_with_dictionary {
                    use super::*;

                    #[test]
                    fn then_returns_dictionary() {
                        let raw = r#"
                            ndict = { foo = "bar" }
                        "#;
                        let mut target = target!(raw);

                        let actual = target.read().expect("Read failed");

                        let mut expected = BTreeMap::new();
                        let mut section = Section::new();
                        let mut dict = BTreeMap::new();
                        dict.insert("foo".to_owned(), Value::String("bar".to_owned()));
                        section
                            .dictionary
                            .insert("ndict".to_owned(), Value::Dictionary(dict));
                        expected.insert("root".to_owned(), section);
                        assert_eq!(expected, actual);
                    }
                }

                mod and_root_section_has_dictionary_with_dictionary_with_new_lines {
                    use super::*;

                    #[test]
                    fn then_returns_dictionary() {
                        let raw = r#"
                            R75042 = {
                            view = "SV"
                            loc  = ["M", "B"]
                            dist = { beach_km = 4.1 }
                        }"#;
                        let mut target = target!(raw);

                        let actual = target.read().expect("Read failed");

                        let mut expected = BTreeMap::new();
                        let mut sect = Section::new();
                        let mut dict = BTreeMap::new();
                        dict.insert("view".to_owned(), Value::String("SV".to_owned()));
                        let array =
                            vec![Value::String("M".to_owned()), Value::String("B".to_owned())];
                        dict.insert("loc".to_owned(), Value::Array(array));
                        let mut dict_dict = BTreeMap::new();
                        dict_dict.insert("beach_km".to_owned(), Value::Float(4.1));
                        dict.insert("dist".to_owned(), Value::Dictionary(dict_dict));
                        sect.dictionary
                            .insert("R75042".to_owned(), Value::Dictionary(dict));
                        expected.insert("root".to_owned(), sect);
                        assert_eq!(expected, actual);
                    }
                }

                mod and_root_section_has_dictionary_with_field_without_value {
                    use super::*;

                    #[test]
                    fn then_returns_error() {
                        let raw = r#"
                            key =
                        "#;
                        let mut target = target!(raw);
                        let actual = ext_err!(target.read());

                        assert_eq!(
                            "ParserError { section: \"unknown\", desc: \"Cannot read a value\" }",
                            actual.to_string()
                        );
                    }
                }

                mod and_root_section_has_array {
                    use super::*;

                    #[test]
                    fn then_returns_array() {
                        let raw = r#"
                            |1|2|
                            |3|
                        "#;
                        let mut target = target!(raw);

                        let actual = target.read().expect("Read failed");

                        let mut expected = BTreeMap::new();
                        let mut sect = Section::new();
                        sect.rows.push(vec![
                            Value::String("1".to_owned()),
                            Value::String("2".to_owned()),
                        ]);
                        sect.rows.push(vec![Value::String("3".to_owned())]);
                        expected.insert("root".to_owned(), sect);
                        assert_eq!(expected, actual);
                    }
                }

                mod and_root_section_has_array_with_empty_cell {
                    use super::*;

                    #[test]
                    fn then_returns_array_with_empty_strings_on_empty_cells() {
                        let raw = r#"
                            |1||2|
                            |3|   |
                        "#;
                        let mut target = target!(raw);

                        let actual = target.read().expect("Read failed");

                        let mut expected = BTreeMap::new();
                        let mut sect = Section::new();
                        sect.rows.push(vec![
                            Value::String("1".to_owned()),
                            Value::String("".to_owned()),
                            Value::String("2".to_owned()),
                        ]);
                        sect.rows.push(vec![
                            Value::String("3".to_owned()),
                            Value::String("".to_owned()),
                        ]);
                        expected.insert("root".to_owned(), sect);
                        assert_eq!(expected, actual);
                    }
                }
            }

            mod and_ion_has_section {
                use super::*;

                mod and_section_occurs_once {
                    use super::*;

                    #[test]
                    fn then_returns_section() {
                        let raw = r#"
                            [SECTION]

                            key = "value"
                            # now a table
                            | col1 | col2|
                            | col1 | col2| # comment
                            | col1 | col2|
                        "#;

                        let expected = {
                            let mut map = BTreeMap::new();
                            let mut section = Section::new();
                            section
                                .dictionary
                                .insert("key".to_owned(), Value::String("value".to_owned()));
                            let mut row = Vec::new();
                            row.push(Value::String("col1".to_owned()));
                            row.push(Value::String("col2".to_owned()));
                            section.rows.push(row.clone());
                            section.rows.push(row.clone());
                            section.rows.push(row);
                            map.insert("SECTION".to_owned(), section);
                            map
                        };

                        let mut target = target!(raw);
                        assert_eq!(expected, target.read().unwrap());
                    }
                }

                mod and_section_is_duplicated {
                    use super::*;

                    #[test]
                    fn then_returns_last_occurance_of_section() {
                        let raw = r#"
                            [SECTION]
                            1key = "1value"
                            | 1col1 | 1col2|
                            [SECTION]
                            2key = "2value"
                            | 2col1 | 2col2|
                        "#;
                        let mut target = target!(raw);

                        let actual = target.read().expect("Read failed");

                        let mut expected = BTreeMap::new();
                        let mut section = Section::new();
                        section
                            .dictionary
                            .insert("2key".to_owned(), Value::String("2value".to_owned()));
                        section.rows.push(vec![
                            Value::String("2col1".to_string()),
                            Value::String("2col2".to_string()),
                        ]);
                        expected.insert("SECTION".to_owned(), section);
                        assert_eq!(expected, actual);
                    }
                }
            }
        }

        mod when_parsing_with_filtering {
            use super::*;

            mod and_ion_has_root_section {
                use super::*;

                mod and_no_other_sections {
                    use super::*;

                    #[test]
                    fn then_returns_nothing() {
                        let raw = r#"
                            nkey = "nvalue"
                            | ncol1 | ncol2 |
                        "#;
                        let mut target = Parser::new_filtered(raw, vec!["ACCEPTED"]);

                        let actual = target.read().expect("Read failed");

                        let expected = BTreeMap::new();
                        assert_eq!(expected, actual);
                    }
                }

                mod and_then_accepted_section {
                    use super::*;

                    #[test]
                    fn then_returns_accepted_section() {
                        let raw = r#"
                            nkey = "nvalue"
                            | ncol1 | ncol2 |
                            [ACCEPTED]
                            key = "value"
                            | col1 | col2|
                        "#;
                        let mut target = Parser::new_filtered(raw, vec!["ACCEPTED"]);

                        let actual = target.read().expect("Read failed");

                        let mut expected = BTreeMap::new();
                        let mut section = Section::new();
                        section
                            .dictionary
                            .insert("key".to_owned(), Value::String("value".to_owned()));
                        section.rows.push(vec![
                            Value::String("col1".to_string()),
                            Value::String("col2".to_string()),
                        ]);
                        expected.insert("ACCEPTED".to_owned(), section);
                        assert_eq!(expected, actual);
                    }
                }

                mod and_then_filtered_section {
                    use super::*;

                    #[test]
                    fn then_returns_nothing() {
                        let raw = r#"
                            nkey = "nvalue"
                            | ncol1 | ncol2 |
                            [FILTERED]
                            key = "value"
                            | col1 | col2|
                        "#;
                        let mut target = Parser::new_filtered(raw, vec!["ACCEPTED"]);

                        let actual = target.read().expect("Read failed");

                        let expected = BTreeMap::new();
                        assert_eq!(expected, actual);
                    }
                }
            }

            mod and_ion_has_accepted_section {
                use super::*;

                mod and_no_other_sections {
                    use super::*;

                    #[test]
                    fn then_returns_accepted_section() {
                        let raw = r#"
                            [ACCEPTED]
                            key = "value"
                            | col1 | col2|
                        "#;
                        let mut target = Parser::new_filtered(raw, vec!["ACCEPTED"]);

                        let actual = target.read().expect("Read failed");

                        let mut expected = BTreeMap::new();
                        let mut section = Section::new();
                        section
                            .dictionary
                            .insert("key".to_owned(), Value::String("value".to_owned()));
                        section.rows.push(vec![
                            Value::String("col1".to_string()),
                            Value::String("col2".to_string()),
                        ]);
                        expected.insert("ACCEPTED".to_owned(), section);
                        assert_eq!(expected, actual);
                    }
                }

                mod and_then_filtered_section {
                    use super::*;

                    #[test]
                    fn then_returns_accepted_section() {
                        let raw = r#"
                            [ACCEPTED]
                            key = "value"
                            | col1 | col2|
                            [FILTERED]
                            fkey = "fvalue"
                            | fcol1 | fcol2|
                        "#;
                        let mut target = Parser::new_filtered(raw, vec!["ACCEPTED"]);

                        let actual = target.read().expect("Read failed");

                        let mut expected = BTreeMap::new();
                        let mut section = Section::new();
                        section
                            .dictionary
                            .insert("key".to_owned(), Value::String("value".to_owned()));
                        section.rows.push(vec![
                            Value::String("col1".to_string()),
                            Value::String("col2".to_string()),
                        ]);
                        expected.insert("ACCEPTED".to_owned(), section);
                        assert_eq!(expected, actual);
                    }
                }

                mod and_then_duplicated_allowed_section {
                    use super::*;

                    mod and_it_is_the_only_accepted_section {
                        use super::*;

                        #[test]
                        fn then_returns_first_occurance_of_accepted_section() {
                            let raw = r#"
                                [ACCEPTED]
                                1key = "1value"
                                | 1col1 | 1col2|
                                [ACCEPTED]
                                2key = "2value"
                                | 2col1 | 2col2|
                            "#;
                            let mut target = Parser::new_filtered(raw, vec!["ACCEPTED"]);

                            let actual = target.read().expect("Read failed");

                            let mut expected = BTreeMap::new();
                            let mut section = Section::new();
                            section
                                .dictionary
                                .insert("1key".to_owned(), Value::String("1value".to_owned()));
                            section.rows.push(vec![
                                Value::String("1col1".to_string()),
                                Value::String("1col2".to_string()),
                            ]);
                            expected.insert("ACCEPTED".to_owned(), section);
                            assert_eq!(expected, actual);
                        }
                    }

                    mod and_it_is_not_the_only_accepted_section {
                        use super::*;

                        #[test]
                        fn then_returns_first_occurance_of_accepted_section() {
                            let raw = r#"
                                [ACCEPTED]
                                1key = "1value"
                                | 1col1 | 1col2|
                                [ACCEPTED]
                                2key = "2value"
                                | 2col1 | 2col2|
                            "#;
                            let mut target = Parser::new_filtered(raw, vec!["ACCEPTED", "ANOTHER"]);

                            let actual = target.read().expect("Read failed");

                            let mut expected = BTreeMap::new();
                            let mut section = Section::new();
                            section
                                .dictionary
                                .insert("1key".to_owned(), Value::String("1value".to_owned()));
                            section.rows.push(vec![
                                Value::String("1col1".to_string()),
                                Value::String("1col2".to_string()),
                            ]);
                            expected.insert("ACCEPTED".to_owned(), section);
                            assert_eq!(expected, actual);
                        }
                    }
                }
            }

            mod and_ion_has_filtered_section {
                use super::*;

                mod and_no_other_sections {
                    use super::*;

                    #[test]
                    fn then_returns_nothing() {
                        let raw = r#"
                            [FILTERED]
                            key = "value"
                            | col1 | col2|
                        "#;
                        let mut target = Parser::new_filtered(raw, vec!["ACCEPTED"]);

                        let actual = target.read().expect("Read failed");

                        let expected = BTreeMap::new();
                        assert_eq!(expected, actual);
                    }
                }

                mod and_then_accepted_section {
                    use super::*;

                    #[test]
                    fn then_returns_accepted_section() {
                        let raw = r#"
                            [FILTERED]
                            fkey = "fvalue"
                            | fcol1 | fcol2|
                            [ACCEPTED]
                            key = "value"
                            | col1 | col2|
                        "#;
                        let mut target = Parser::new_filtered(raw, vec!["ACCEPTED"]);

                        let actual = target.read().expect("Read failed");

                        let mut expected = BTreeMap::new();
                        let mut section = Section::new();
                        section
                            .dictionary
                            .insert("key".to_owned(), Value::String("value".to_owned()));
                        section.rows.push(vec![
                            Value::String("col1".to_string()),
                            Value::String("col2".to_string()),
                        ]);
                        expected.insert("ACCEPTED".to_owned(), section);
                        assert_eq!(expected, actual);
                    }
                }
            }
        }
    }
}
