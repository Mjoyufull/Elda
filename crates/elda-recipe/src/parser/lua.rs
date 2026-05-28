use std::collections::BTreeMap;

use crate::error::RecipeError;
use crate::model::LuaValue;

pub(super) struct LuaParser<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> LuaParser<'a> {
    pub(super) fn new(input: &'a str) -> Self {
        Self { input, cursor: 0 }
    }

    pub(super) fn parse_document(&mut self) -> Result<BTreeMap<String, LuaValue>, RecipeError> {
        self.skip_ws_and_comments();
        let identifier = self.parse_identifier()?;
        if identifier != "pkg" {
            return Err(RecipeError::Parse(
                "expected a top-level `pkg = { ... }` assignment".to_owned(),
            ));
        }
        self.skip_ws_and_comments();
        self.expect('=')?;
        self.skip_ws_and_comments();

        match self.parse_value()? {
            LuaValue::Table(table) => Ok(table),
            _ => Err(RecipeError::Parse(
                "top-level `pkg` assignment must be a table".to_owned(),
            )),
        }
    }

    fn parse_value(&mut self) -> Result<LuaValue, RecipeError> {
        self.skip_ws_and_comments();
        match self.peek_char() {
            Some('"') | Some('\'') => Ok(LuaValue::String(self.parse_string()?)),
            Some('{') => self.parse_table(),
            Some(character) if character.is_ascii_digit() || character == '-' => {
                Ok(LuaValue::Integer(self.parse_integer()?))
            }
            Some(character) if is_identifier_start(character) => {
                let identifier = self.parse_identifier()?;
                match identifier.as_str() {
                    "true" => Ok(LuaValue::Boolean(true)),
                    "false" => Ok(LuaValue::Boolean(false)),
                    _ => Err(RecipeError::Parse(format!(
                        "unexpected bare identifier `{identifier}` in declarative pkg.lua subset"
                    ))),
                }
            }
            Some(other) => Err(RecipeError::Parse(format!(
                "unexpected character `{other}` while parsing pkg.lua"
            ))),
            None => Err(RecipeError::Parse("unexpected end of file".to_owned())),
        }
    }

    fn parse_table(&mut self) -> Result<LuaValue, RecipeError> {
        self.expect('{')?;
        self.skip_ws_and_comments();

        let mut entries = Vec::new();
        let mut saw_keyed = false;
        let mut saw_array = false;

        while !self.try_consume('}') {
            self.skip_ws_and_comments();

            if self.peek_is_identifier_key() {
                saw_keyed = true;
                if saw_array {
                    return Err(RecipeError::Parse(
                        "mixed keyed and array table entries are not supported".to_owned(),
                    ));
                }
                let key = self.parse_identifier()?;
                self.skip_ws_and_comments();
                self.expect('=')?;
                self.skip_ws_and_comments();
                let value = self.parse_value()?;
                entries.push((Some(key), value));
            } else {
                saw_array = true;
                if saw_keyed {
                    return Err(RecipeError::Parse(
                        "mixed keyed and array table entries are not supported".to_owned(),
                    ));
                }
                let value = self.parse_value()?;
                entries.push((None, value));
            }

            self.skip_ws_and_comments();
            if self.try_consume(',') {
                self.skip_ws_and_comments();
            }
        }

        if saw_keyed {
            let mut table = BTreeMap::new();
            for (key, value) in entries {
                let key = key.ok_or_else(|| {
                    RecipeError::Parse("internal parser mismatch for keyed table".to_owned())
                })?;
                table.insert(key, value);
            }
            Ok(LuaValue::Table(table))
        } else {
            Ok(LuaValue::Array(
                entries.into_iter().map(|(_, value)| value).collect(),
            ))
        }
    }

    fn parse_identifier(&mut self) -> Result<String, RecipeError> {
        let mut identifier = String::new();

        let Some(character) = self.peek_char() else {
            return Err(RecipeError::Parse(
                "unexpected end of file while parsing identifier".to_owned(),
            ));
        };
        if !is_identifier_start(character) {
            return Err(RecipeError::Parse(format!(
                "unexpected character `{character}` while parsing identifier"
            )));
        }

        identifier.push(character);
        self.cursor += character.len_utf8();

        while let Some(character) = self.peek_char() {
            if is_identifier_continue(character) {
                identifier.push(character);
                self.cursor += character.len_utf8();
            } else {
                break;
            }
        }

        Ok(identifier)
    }

    fn parse_string(&mut self) -> Result<String, RecipeError> {
        let quote = self.next_char().ok_or_else(|| {
            RecipeError::Parse("unexpected end of file while parsing string".to_owned())
        })?;
        let mut value = String::new();

        while let Some(character) = self.next_char() {
            if character == quote {
                return Ok(value);
            }
            if character == '\\' {
                let escaped = self.next_char().ok_or_else(|| {
                    RecipeError::Parse("unterminated escape sequence in string".to_owned())
                })?;
                value.push(match escaped {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '"' => '"',
                    '\'' => '\'',
                    '\\' => '\\',
                    other => other,
                });
            } else {
                value.push(character);
            }
        }

        Err(RecipeError::Parse("unterminated string literal".to_owned()))
    }

    fn parse_integer(&mut self) -> Result<i64, RecipeError> {
        let mut value = String::new();

        if self.try_consume('-') {
            value.push('-');
        }

        while let Some(character) = self.peek_char() {
            if character.is_ascii_digit() {
                value.push(character);
                self.cursor += character.len_utf8();
            } else {
                break;
            }
        }

        value
            .parse::<i64>()
            .map_err(|_| RecipeError::Parse(format!("invalid integer literal `{value}`")))
    }

    fn peek_is_identifier_key(&self) -> bool {
        let mut cursor = self.cursor;
        skip_ws_and_comments_from(self.input, &mut cursor);
        let Some(character) = self.input[cursor..].chars().next() else {
            return false;
        };
        if !is_identifier_start(character) {
            return false;
        }

        cursor += character.len_utf8();
        while let Some(character) = self.input[cursor..].chars().next() {
            if is_identifier_continue(character) {
                cursor += character.len_utf8();
            } else {
                break;
            }
        }
        skip_ws_and_comments_from(self.input, &mut cursor);

        self.input[cursor..].starts_with('=')
    }

    fn skip_ws_and_comments(&mut self) {
        skip_ws_and_comments_from(self.input, &mut self.cursor);
    }

    fn expect(&mut self, expected: char) -> Result<(), RecipeError> {
        let found = self.next_char().ok_or_else(|| {
            RecipeError::Parse(format!("expected `{expected}` but reached end of file"))
        })?;
        if found == expected {
            Ok(())
        } else {
            Err(RecipeError::Parse(format!(
                "expected `{expected}` but found `{found}`"
            )))
        }
    }

    fn try_consume(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.cursor += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    fn next_char(&mut self) -> Option<char> {
        let character = self.peek_char()?;
        self.cursor += character.len_utf8();
        Some(character)
    }
}

fn skip_ws_and_comments_from(input: &str, cursor: &mut usize) {
    loop {
        let remaining = &input[*cursor..];
        if remaining.starts_with("--") {
            while let Some(character) = input[*cursor..].chars().next() {
                *cursor += character.len_utf8();
                if character == '\n' {
                    break;
                }
            }
            continue;
        }

        let mut consumed = false;
        while let Some(character) = input[*cursor..].chars().next() {
            if character.is_whitespace() {
                *cursor += character.len_utf8();
                consumed = true;
            } else {
                break;
            }
        }

        if !consumed {
            break;
        }
    }
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    is_identifier_start(character) || character.is_ascii_digit()
}
