use std::{mem, num::ParseIntError};

use thiserror::Error;

use crate::warn::{OptionExt, ResultExt, Sink};

use super::{tok, EolError};

use super::super::{
    msg::OptBody,
    str::{self, OptComboVar, UciString, UciToken},
};

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum Error {
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error(transparent)]
    UnexpectedEol(#[from] EolError),
    #[error("extra token: {0}")]
    ExtraToken(String),
    #[error("unknown type \"{0}\"")]
    UnknownType(String),
    #[error("expected \"{0}\" token")]
    ExpectedToken(&'static str),
    #[error("expected \"true\" or \"false\" token")]
    ExpectedBool,
    #[error("cannot parse integer: {0}")]
    BadInteger(#[from] ParseIntError),
    #[error("cannot convert string to default combo variant: {0}")]
    BadComboDefaultVar(#[source] str::Error),
    #[error("cannot convert string to combo variant {}: {}", pos + 1, error)]
    BadComboVar {
        pos: usize,
        #[source]
        error: str::Error,
    },
}

pub fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Error>) -> Option<OptBody> {
    let result = (|| match tok::next_warn(tokens, warn)?.as_str() {
        "check" => {
            tok::expect(tokens, "default", Error::ExpectedToken("default"), warn)?;
            let value = match tok::next_warn(tokens, warn)?.as_str() {
                "true" => true,
                "false" => false,
                _ => {
                    warn.warn(Error::ExpectedBool);
                    return None;
                }
            };
            Some(OptBody::Check(value))
        }
        "spin" => {
            tok::expect(tokens, "default", Error::ExpectedToken("default"), warn)?;
            let default = tok::parse(tokens, warn)?;
            tok::expect(tokens, "min", Error::ExpectedToken("min"), warn)?;
            let min = tok::parse(tokens, warn)?;
            tok::expect(tokens, "max", Error::ExpectedToken("max"), warn)?;
            let max = tok::parse(tokens, warn)?;
            Some(OptBody::Spin { default, min, max })
        }
        "combo" => {
            tok::expect(tokens, "default", Error::ExpectedToken("default"), warn)?;
            let mut iter = tokens.split(|&tok| tok == "var").fuse();
            *tokens = &[];
            let default = iter
                .next()
                .or_warn_with(Error::UnexpectedEol(EolError), warn)
                .unwrap_or(&[]);
            let default = OptComboVar::from_tokens(default)
                .or_warn_map(Error::BadComboDefaultVar, warn)
                .ok()?;
            let vars: Vec<_> = iter
                .enumerate()
                .filter_map(|(pos, toks)| {
                    OptComboVar::from_tokens(toks)
                        .or_warn_map(|error| Error::BadComboVar { pos, error }, warn)
                        .ok()
                })
                .collect();
            Some(OptBody::Combo { default, vars })
        }
        "button" => Some(OptBody::Button),
        "string" => {
            tok::expect(tokens, "default", Error::ExpectedToken("default"), warn)?;
            Some(OptBody::String(UciString::from_tokens(mem::take(tokens))))
        }
        tok => {
            warn.warn(Error::UnknownType(tok.to_string()));
            None
        }
    })();
    if !tokens.is_empty() {
        warn.warn(Error::ExtraToken(tokens[0].to_string()));
    }
    result
}
