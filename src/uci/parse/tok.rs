use std::{error::Error, str::FromStr};

use super::EolError;

use super::super::str::UciToken;

use crate::warn::{OptionExt, ResultExt, Sink};

pub fn try_split<'a, 'b>(
    src: &'a [&'b UciToken],
    mid: &str,
) -> (&'a [&'b UciToken], Option<&'a [&'b UciToken]>) {
    match src.iter().position(|v| *v == mid) {
        Some(pos) => (&src[..pos], Some(&src[pos + 1..])),
        None => (src, None),
    }
}

pub fn split<'a, 'b, E: Error>(
    src: &'a [&'b UciToken],
    mid: &str,
    error: E,
    warn: &mut impl Sink<E>,
) -> (&'a [&'b UciToken], &'a [&'b UciToken]) {
    let (l, r) = try_split(src, mid);
    (l, r.or_warn_with(error, warn).unwrap_or(&[]))
}

pub fn next<'a>(tokens: &mut &[&'a UciToken]) -> Option<&'a UciToken> {
    let result;
    (result, *tokens) = tokens.split_first()?;
    Some(*result)
}

pub fn next_warn<'a, E: From<EolError> + Error>(
    tokens: &mut &[&'a UciToken],
    warn: &mut impl Sink<E>,
) -> Option<&'a UciToken> {
    next(tokens).or_warn_with(EolError.into(), warn)
}

pub fn parse<D, E, T>(tokens: &mut &[&UciToken], warn: &mut impl Sink<E>) -> Option<T>
where
    D: Error + Clone,
    E: From<D> + From<EolError> + Error,
    T: FromStr<Err = D>,
{
    parse_map(tokens, From::from, warn)
}

pub fn parse_map<D, E, F, T>(
    tokens: &mut &[&UciToken],
    func: F,
    warn: &mut impl Sink<E>,
) -> Option<T>
where
    D: Error + Clone,
    E: From<EolError> + Error,
    F: FnOnce(D) -> E,
    T: FromStr<Err = D>,
{
    next_warn(tokens, warn)?
        .as_str()
        .parse()
        .or_warn_map(func, warn)
        .ok()
}

pub fn expect<E: From<EolError> + Error>(
    tokens: &mut &[&UciToken],
    expected: &str,
    on_mismatch: E,
    warn: &mut impl Sink<E>,
) -> Option<()> {
    if next_warn(tokens, warn)? != expected {
        warn.warn(on_mismatch);
        return None;
    }
    Some(())
}
