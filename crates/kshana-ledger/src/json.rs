//! A flat JSON object, written and read by hand: enough for one event or one sealed
//! forecast per line. Strings, numbers, booleans, null and arrays of those are
//! understood; nested objects and nested arrays are rejected, since no ledger line
//! needs them.

/// A JSON value: a scalar, or an array of scalars.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Value {
    /// A string.
    Str(String),
    /// A number.
    Num(f64),
    /// `true` or `false`.
    Bool(bool),
    /// `null`.
    Null,
    /// An array of scalars.
    List(Vec<Value>),
}

impl Value {
    /// The strings of a list (non-strings skipped), or an empty list for anything else.
    pub(crate) fn strings(&self) -> Vec<String> {
        match self {
            Value::List(items) => items
                .iter()
                .filter_map(|v| match v {
                    Value::Str(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

fn write_scalar(out: &mut String, v: &Value) {
    match v {
        Value::Str(s) => push_string(out, s),
        Value::Num(n) if n.is_finite() => out.push_str(&format!("{n}")),
        Value::Num(_) | Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::List(_) => out.push_str("null"),
    }
}

/// Append `s` to `out` as a quoted, escaped JSON string.
pub(crate) fn push_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Write a flat object with its fields in the given order, on one line.
pub(crate) fn write_object(fields: &[(&str, Value)]) -> String {
    let mut out = String::from("{");
    for (i, (k, v)) in fields.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_string(&mut out, k);
        out.push(':');
        match v {
            Value::List(items) => {
                out.push('[');
                for (j, item) in items.iter().enumerate() {
                    if j > 0 {
                        out.push(',');
                    }
                    write_scalar(&mut out, item);
                }
                out.push(']');
            }
            scalar => write_scalar(&mut out, scalar),
        }
    }
    out.push('}');
    out
}

struct Cursor<'a> {
    s: &'a str,
    b: &'a [u8],
    i: usize,
}

impl Cursor<'_> {
    fn skip_ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected '{}' at byte {}", c as char, self.i))
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let h = self
            .s
            .get(self.i..self.i + 4)
            .ok_or_else(|| "short \\u escape".to_string())?;
        let v = u32::from_str_radix(h, 16).map_err(|_| format!("bad \\u escape {h:?}"))?;
        self.i += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let Some(c) = self.peek() else {
                return Err("unterminated string".into());
            };
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(e) = self.peek() else {
                        return Err("unterminated escape".into());
                    };
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                if self.s[self.i..].starts_with("\\u") {
                                    self.i += 2;
                                    let lo = self.hex4()?;
                                    if (0xDC00..0xE000).contains(&lo) {
                                        0x1_0000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                                    } else {
                                        0xFFFD
                                    }
                                } else {
                                    0xFFFD
                                }
                            } else {
                                hi
                            };
                            out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                        }
                        other => return Err(format!("bad escape \\{}", other as char)),
                    }
                }
                _ => {
                    // Copy the whole UTF-8 character that starts at this byte.
                    let start = self.i - 1;
                    let ch = self.s[start..]
                        .chars()
                        .next()
                        .ok_or_else(|| "bad utf-8".to_string())?;
                    out.push(ch);
                    self.i = start + ch.len_utf8();
                }
            }
        }
    }

    fn literal(&mut self, word: &str, v: Value) -> Result<Value, String> {
        if self.s[self.i..].starts_with(word) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(format!("bad literal at byte {}", self.i))
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.i;
        while self.i < self.b.len()
            && matches!(
                self.b[self.i],
                b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E'
            )
        {
            self.i += 1;
        }
        let t = &self.s[start..self.i];
        t.parse::<f64>()
            .map(Value::Num)
            .map_err(|_| format!("bad number {t:?} at byte {start}"))
    }

    fn scalar(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b'{') | Some(b'[') => Err("nested values are not supported".into()),
            Some(_) => self.number(),
            None => Err("unexpected end of line".into()),
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        if self.peek() != Some(b'[') {
            return self.scalar();
        }
        self.i += 1;
        self.skip_ws();
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Value::List(items));
        }
        loop {
            self.skip_ws();
            items.push(self.scalar()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(Value::List(items));
                }
                _ => return Err(format!("expected ',' or ']' at byte {}", self.i)),
            }
        }
    }
}

/// Read one flat object. Keys keep their order; duplicates are kept as read.
pub(crate) fn parse_object(s: &str) -> Result<Vec<(String, Value)>, String> {
    let mut c = Cursor {
        s,
        b: s.as_bytes(),
        i: 0,
    };
    c.skip_ws();
    c.expect(b'{')?;
    c.skip_ws();
    let mut out = Vec::new();
    if c.peek() == Some(b'}') {
        c.i += 1;
    } else {
        loop {
            c.skip_ws();
            let k = c.string()?;
            c.skip_ws();
            c.expect(b':')?;
            c.skip_ws();
            let v = c.value()?;
            out.push((k, v));
            c.skip_ws();
            match c.peek() {
                Some(b',') => c.i += 1,
                Some(b'}') => {
                    c.i += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or '}}' at byte {}", c.i)),
            }
        }
    }
    c.skip_ws();
    if c.i != c.b.len() {
        return Err("trailing characters after the object".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_reads_every_scalar() {
        let line = write_object(&[
            ("s", Value::Str("a \"q\" \\ \n\t\u{1} é 😀".into())),
            ("n", Value::Num(0.8)),
            ("one", Value::Num(1.0)),
            ("b", Value::Bool(true)),
            ("z", Value::Null),
            ("nan", Value::Num(f64::NAN)),
        ]);
        assert!(line.starts_with("{\"s\":\"a \\\"q\\\" \\\\ \\n\\t\\u0001 é 😀\""));
        assert!(line.contains("\"n\":0.8"));
        assert!(line.contains("\"one\":1"));
        assert!(line.ends_with("\"b\":true,\"z\":null,\"nan\":null}"));
        let back = parse_object(&line).unwrap();
        assert_eq!(back.len(), 6);
        assert_eq!(back[0].1, Value::Str("a \"q\" \\ \n\t\u{1} é 😀".into()));
        assert_eq!(back[1].1, Value::Num(0.8));
        assert_eq!(back[3].1, Value::Bool(true));
        assert_eq!(back[4].1, Value::Null);
        assert_eq!(back[5].1, Value::Null);
    }

    #[test]
    fn reads_escapes_whitespace_and_empty_objects() {
        let v =
            parse_object(" { \"k\" : \"\\u00e9\\ud83d\\ude00\\/\" , \"n\" : -1.5e2 } ").unwrap();
        assert_eq!(v[0].1, Value::Str("é😀/".into()));
        assert_eq!(v[1].1, Value::Num(-150.0));
        assert!(parse_object("{}").unwrap().is_empty());
        let list = parse_object("{\"a\":[1, \"x\" ,true,null],\"e\":[]}").unwrap();
        assert_eq!(
            list[0].1,
            Value::List(vec![
                Value::Num(1.0),
                Value::Str("x".into()),
                Value::Bool(true),
                Value::Null
            ])
        );
        assert_eq!(list[0].1.strings(), vec!["x".to_string()]);
        assert_eq!(list[1].1, Value::List(Vec::new()));
        assert!(Value::Null.strings().is_empty());
        let written = write_object(&[(
            "b",
            Value::List(vec![Value::Str("p \"q\"".into()), Value::Num(2.0)]),
        )]);
        assert_eq!(written, "{\"b\":[\"p \\\"q\\\"\",2]}");
        assert_eq!(
            parse_object(&written).unwrap()[0].1.strings(),
            vec!["p \"q\"".to_string()]
        );
        assert!(parse_object("{\"a\":[[1]]}").is_err());
        assert!(parse_object("{\"a\":[{}]}").is_err());
        assert!(parse_object("{\"a\":[1,]}").is_err());
        assert!(parse_object("{\"a\":[1").is_err());
        assert!(parse_object("{\"a\":1} x").is_err());
        assert!(parse_object("{\"a\":\"open}").is_err());
        assert!(parse_object("{\"a\":tru}").is_err());
        assert!(parse_object("{\"a\":1,}").is_err());
    }
}
