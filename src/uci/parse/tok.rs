use std::{error::Error, fmt, str::FromStr};

use wurm::{OptionExt, ResultExt, Warn};

use super::super::token::{MultiTokenSafe, PushTokens, Token, TokenSafe};
use super::EolError;

pub fn try_split<'a, 'b>(
    src: &'a [&'b Token],
    mid: &str,
) -> (&'a [&'b Token], Option<&'a [&'b Token]>) {
    match src.iter().position(|v| *v == mid) {
        Some(pos) => (&src[..pos], Some(&src[pos + 1..])),
        None => (src, None),
    }
}

pub fn split<'a, 'b, E: Error>(
    src: &'a [&'b Token],
    mid: &str,
    error: E,
    warn: &mut impl Warn<E>,
) -> (&'a [&'b Token], &'a [&'b Token]) {
    let (l, r) = try_split(src, mid);
    (l, r.or_warn_with(error, warn).unwrap_or(&[]))
}

pub fn next<'a>(tokens: &mut &[&'a Token]) -> Option<&'a Token> {
    let result;
    (result, *tokens) = tokens.split_first()?;
    Some(*result)
}

pub fn next_warn<'a, E: From<EolError> + Error>(
    tokens: &mut &[&'a Token],
    warn: &mut impl Warn<E>,
) -> Option<&'a Token> {
    next(tokens).or_warn_with(EolError.into(), warn)
}

pub fn parse<D, E, T>(tokens: &mut &[&Token], warn: &mut impl Warn<E>) -> Option<T>
where
    D: Error,
    E: From<D> + From<EolError> + Error,
    T: FromStr<Err = D>,
{
    parse_map(tokens, From::from, warn)
}

pub fn parse_map<D, E, F, T>(tokens: &mut &[&Token], func: F, warn: &mut impl Warn<E>) -> Option<T>
where
    D: Error,
    E: From<EolError> + Error,
    F: FnOnce(D) -> E,
    T: FromStr<Err = D>,
{
    next_warn(tokens, warn)?
        .parse()
        .or_warn_map(func, warn)
}

pub fn expect<E: From<EolError> + Error>(
    tokens: &mut &[&Token],
    expected: &str,
    on_mismatch: E,
    warn: &mut impl Warn<E>,
) -> Option<()> {
    if next_warn(tokens, warn)? != expected {
        warn.warn(on_mismatch);
        return None;
    }
    Some(())
}

struct Kw(&'static str);

unsafe impl TokenSafe for Kw {}

impl fmt::Display for Kw {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub trait PushTokensExt: PushTokens {
    #[inline]
    fn push_kw(&mut self, kw: &'static str) {
        self.push_fmt(&Kw(kw));
    }

    #[inline]
    fn push_tag<T: TokenSafe>(&mut self, key: &'static str, value: &T) {
        self.push_kw(key);
        self.push_fmt(value);
    }

    #[inline]
    fn push_tag_many<T: MultiTokenSafe>(&mut self, key: &'static str, value: &T) {
        self.push_kw(key);
        self.push_many_fmt(value);
    }
}

impl<T: PushTokens> PushTokensExt for T {}
