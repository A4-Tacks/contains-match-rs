#![doc = include_str!("../README.md")]
#![no_std]
#![warn(clippy::pedantic)]
#![allow(clippy::cast_possible_truncation)]

use core::{error::Error, fmt::Display, mem::size_of};

use alloc::{borrow::Cow, string::String, vec::Vec};
extern crate alloc;

/// Checked 'contains match' pattern.
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pattern(Repr, Cow<'static, str>);

impl Pattern {
    /// Create pattern from 'contains match' syntax, see [crate](self) docs.
    ///
    /// # Errors
    ///
    /// This function will return an error if syntax error.
    pub fn new(s: impl Into<String>) -> Result<Self, ParseError> {
        let s = s.into();
        Ok(Self(Repr::of(&s)?, s.into()))
    }

    /// Create pattern from 'contains match' syntax, see [crate](self) docs.
    ///
    /// # Errors
    ///
    /// This function will return an error if syntax error.
    pub const fn new_static(s: &'static str) -> Result<Self, ParseError> {
        match Repr::of(s) {
            Ok(it) => Ok(Self(it, Cow::Borrowed(s))),
            Err(e) => Err(e),
        }
    }

    /// Test this [`Pattern`] is matched of given [`str`].
    ///
    /// # Examples
    ///
    /// ```
    /// use contains_match::Pattern;
    ///
    /// let pattern = Pattern::new("foo*").unwrap();
    /// assert_eq!(pattern.is_match("foobar"), true);
    /// assert_eq!(pattern.is_match("barfoo"), false);
    /// ```
    // NOTE: 不要const, 不然就没有编译器内部优化字符串比较了
    #[must_use]
    pub fn is_match(&self, s: &str) -> bool {
        let p = &self.1;
        match self.0 {
            Repr::StartsWith => s.starts_with(&p[..p.len() - 1]),
            Repr::EndsWith => s.ends_with(&p[1..]),
            Repr::Both(at) => {
                let (start, end) = split_of(s, at as usize);
                s.starts_with(start) && s.ends_with(end)
            }
            Repr::Contains => s.contains(&**p),
            Repr::Exact => *s == p[1..p.len() - 1],
        }
    }
}

impl core::fmt::Debug for Pattern {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // 故意不继承格式化参数避免换行
        let s = &self.1;
        write!(f, "Pattern::")?;
        if let Repr::Both(at) = self.0 {
            return write!(
                f,
                "Both({:?}, {:?})",
                &s[..at as usize],
                &s[at as usize + 1..]
            );
        }
        write!(f, "{:?}", self.0)?;
        match self.0 {
            Repr::StartsWith => write!(f, "({:?})", &s[..s.len() - 1]),
            Repr::EndsWith => write!(f, "({:?})", &s[1..]),
            Repr::Both(_) => unreachable!(),
            Repr::Contains => write!(f, "({s:?})"),
            Repr::Exact => write!(f, "({:?})", &s[1..s.len() - 1]),
        }
    }
}

/// [`Pattern`]s set, test any pattern is match.
#[derive(Debug, Default, Hash, PartialEq, Eq)]
pub struct PatternSet(Vec<Pattern>);

impl PatternSet {
    /// Creates a new [`PatternSet`].
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Test any [`Pattern`] [`is_match`](Pattern::is_match).
    ///
    /// # Examples
    ///
    /// ```
    /// use contains_match::*;
    ///
    /// let a = Pattern::new("foo").unwrap();
    /// let b = Pattern::new("bar").unwrap();
    /// let pattern_set = PatternSet::from_iter([a, b]);
    /// assert_eq!(pattern_set.is_match("foo"), true);
    /// assert_eq!(pattern_set.is_match("bar"), true);
    /// assert_eq!(pattern_set.is_match("oth"), false);
    /// ```
    #[must_use]
    pub fn is_match(&self, s: &str) -> bool {
        self.iter().any(|pat| pat.is_match(s))
    }

    /// Add a new [`Pattern`] to set.
    pub fn insert(&mut self, pattern: Pattern) {
        if self.0.binary_search(&pattern).is_ok() {
            return;
        }
        self.0.push(pattern);
        self.orded();
    }

    pub fn iter(&self) -> core::slice::Iter<'_, Pattern> {
        self.0.iter()
    }

    fn orded(&mut self) -> &mut PatternSet {
        self.0.sort();
        self
    }
}
impl FromIterator<Pattern> for PatternSet {
    fn from_iter<I: IntoIterator<Item = Pattern>>(iter: I) -> Self {
        let mut set = Self(iter.into_iter().collect());
        set.orded().0.dedup();
        set
    }
}
impl IntoIterator for PatternSet {
    type Item = Pattern;
    type IntoIter = alloc::vec::IntoIter<Pattern>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<'a> IntoIterator for &'a PatternSet {
    type Item = &'a Pattern;
    type IntoIter = core::slice::Iter<'a, Pattern>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum Repr {
    StartsWith,
    EndsWith,
    Both(u32),
    Contains,
    Exact,
}
impl Repr {
    const fn of(s: &str) -> Result<Repr, ParseError> {
        match s.as_bytes() {
            [b'*', .., b'*'] => Ok(Self::Exact),
            [b'*'] => Ok(Self::Contains),
            [b'*', ..] => Ok(Self::EndsWith),
            [.., b'*'] => Ok(Self::StartsWith),
            _ => {
                let Some(at) = find(s, b'*') else {
                    return Ok(Self::Contains);
                };
                if let Some(err) = find(split_of(s, at).1, b'*') {
                    return Err(ParseError::ExtraStar(at + err + 1));
                }
                debug_assert!(at != 0);
                debug_assert!(at != s.len() - 1);

                if size_of::<usize>() > size_of::<u32>() && at > u32::MAX as usize {
                    return Err(ParseError::PatternTooLong);
                }

                Ok(Self::Both(at as u32))
            }
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    ExtraStar(usize),
    PatternTooLong,
}

impl Error for ParseError {
    fn description(&self) -> &str {
        match self {
            ParseError::ExtraStar(_) => "extra `*` in pattern",
            ParseError::PatternTooLong => "pattern too long",
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseError::ExtraStar(at) => write!(f, "extra `*` in pattern at {at} byte"),
            ParseError::PatternTooLong => write!(f, "pattern too long"),
        }
    }
}

const fn find(s: &str, with: u8) -> Option<usize> {
    let bytes = s.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == with {
            return Some(i);
        }
        i += 1;
    }

    None
}

#[allow(clippy::transmute_bytes_to_str)] // msrv
const fn split_of(s: &str, with: usize) -> (&str, &str) {
    let (a, b) = s.as_bytes().split_at(with);
    assert!(b[0] <= 0x7f, "can't support split multi-bytes char");
    // SAFETY: is valided utf8
    unsafe {
        (
            core::mem::transmute::<&[u8], &str>(a),
            core::mem::transmute::<&[u8], &str>(b.split_at(1).1),
        )
    }
}

#[cfg(test)]
mod tests {
    use test_buffer::test_utils::*;
    use test_buffer::TestBuffer;

    use super::*;

    #[test]
    fn parse() {
        let s = r#"
            text
            a*
            *b
            *
            **
            *a*
            a*b
            foo*bar
            ***
            *some*logic*

            a**b
            a*b*c
        "#;
        let mut buf = TestBuffer::new();
        for pat in s.trim().lines() {
            writeln!(buf, "{:?}", Pattern::new(pat.trim()))
        }
        buf.expect(expect![[r#"
            Ok(Pattern::Contains("text"))
            Ok(Pattern::StartsWith("a"))
            Ok(Pattern::EndsWith("b"))
            Ok(Pattern::Contains("*"))
            Ok(Pattern::Exact(""))
            Ok(Pattern::Exact("a"))
            Ok(Pattern::Both("a", "b"))
            Ok(Pattern::Both("foo", "bar"))
            Ok(Pattern::Exact("*"))
            Ok(Pattern::Exact("some*logic"))
            Ok(Pattern::Contains(""))
            Err(ExtraStar(2))
            Err(ExtraStar(3))
        "#]]);
    }

    #[test]
    fn pattern_set_dedup() {
        let a = Pattern::new("foo").unwrap();
        let b = Pattern::new("bar").unwrap();

        let mut set = PatternSet::from_iter([a.clone(), a.clone()]);
        assert_eq!(set.0.len(), 1);
        set.insert(a);
        assert_eq!(set.0.len(), 1);
        set.insert(b);
        assert_eq!(set.0.len(), 2);
    }
}
