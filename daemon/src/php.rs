//! Minimal PHP serialize() parser (enough for `_wp_attachment_metadata`).
//! Supports: s, i, d, b, N, a (string/int keys) and nested values.

use serde_json::{Map, Value};

pub fn parse(input: &str) -> Option<Value> {
    let mut p = Parser { b: input.as_bytes(), i: 0 };
    p.value()
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn expect(&mut self, c: u8) -> Option<()> {
        if self.peek()? == c {
            self.i += 1;
            Some(())
        } else {
            None
        }
    }

    fn value(&mut self) -> Option<Value> {
        match self.peek()? {
            b's' => self.string().map(Value::String),
            b'i' => self.int().map(|v| Value::from(v)),
            b'd' => self.double().map(Value::from),
            b'b' => self.boolean().map(Value::Bool),
            b'N' => {
                self.i += 1;
                self.expect(b';')?;
                Some(Value::Null)
            }
            b'a' => self.array(),
            _ => None,
        }
    }

    fn string(&mut self) -> Option<String> {
        self.expect(b's')?;
        self.expect(b':')?;
        let len: usize = self.num()?.try_into().ok()?;
        self.expect(b':')?;
        self.expect(b'"')?;
        if self.i + len > self.b.len() {
            return None;
        }
        let s = std::str::from_utf8(&self.b[self.i..self.i + len]).ok()?.to_string();
        self.i += len;
        self.expect(b'"')?;
        self.expect(b';')?;
        Some(s)
    }

    fn num(&mut self) -> Option<i64> {
        let start = self.i;
        if self.peek()? == b'-' {
            self.i += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.i += 1;
        }
        std::str::from_utf8(&self.b[start..self.i]).ok()?.parse().ok()
    }

    fn int(&mut self) -> Option<i64> {
        self.expect(b'i')?;
        self.expect(b':')?;
        let n = self.num()?;
        self.expect(b';')?;
        Some(n)
    }

    fn double(&mut self) -> Option<f64> {
        self.expect(b'd')?;
        self.expect(b':')?;
        let start = self.i;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == b'-' || c == b'.' || c == b'E' || c == b'e' || c == b'+') {
            self.i += 1;
        }
        let n: f64 = std::str::from_utf8(&self.b[start..self.i]).ok()?.parse().ok()?;
        self.expect(b';')?;
        Some(n)
    }

    fn boolean(&mut self) -> Option<bool> {
        self.expect(b'b')?;
        self.expect(b':')?;
        let v = match self.peek()? {
            b'0' => false,
            b'1' => true,
            _ => return None,
        };
        self.i += 1;
        self.expect(b';')?;
        Some(v)
    }

    fn array(&mut self) -> Option<Value> {
        self.expect(b'a')?;
        self.expect(b':')?;
        let count: usize = self.num()?.try_into().ok()?;
        self.expect(b':')?;
        self.expect(b'{')?;
        let mut map = Map::new();
        for _ in 0..count {
            let key = match self.peek()? {
                b's' => self.string()?,
                b'i' => self.int_key()?,
                _ => return None,
            };
            let val = self.value()?;
            map.insert(key, val);
        }
        self.expect(b'}')?;
        Some(Value::Object(map))
    }

    fn int_key(&mut self) -> Option<String> {
        self.expect(b'i')?;
        self.expect(b':')?;
        let start = self.i;
        if self.peek()? == b'-' {
            self.i += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.i += 1;
        }
        let s = std::str::from_utf8(&self.b[start..self.i]).ok()?.to_string();
        self.expect(b';')?;
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wp_metadata() {
        let raw = r#"a:2:{s:5:"width";i:1920;s:5:"sizes";a:1:{s:6:"medium";a:3:{s:5:"width";i:225;s:6:"height";i:300;s:4:"crop";b:0;}}}"#;
        let v = parse(raw).expect("parse");
        assert_eq!(v.get("width").and_then(|x| x.as_i64()), Some(1920));
        let medium = v.get("sizes").and_then(|s| s.get("medium")).expect("medium");
        assert_eq!(medium.get("width").and_then(|x| x.as_i64()), Some(225));
        assert_eq!(medium.get("height").and_then(|x| x.as_i64()), Some(300));
        assert_eq!(medium.get("crop").and_then(|x| x.as_bool()), Some(false));
    }

    #[test]
    fn parses_int_keys_and_nested_arrays() {
        let raw = r#"a:2:{i:0;s:3:"abc";i:1;a:1:{s:3:"foo";d:1.5;}}"#;
        let v = parse(raw).expect("parse");
        assert_eq!(v.get("0").and_then(|x| x.as_str()), Some("abc"));
        assert_eq!(v.get("1").and_then(|x| x.get("foo")).and_then(|x| x.as_f64()), Some(1.5));
    }
}
