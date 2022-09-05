use std::num::ParseIntError;

use thiserror::Error;

use crate::warn::{OptionExt, Sink};

use crate::score::{Bound, BoundedRelScore, RelScore};

use super::{tok, EolError};

use super::super::str::UciToken;

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum Error {
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error(transparent)]
    UnexpectedEol(#[from] EolError),
    #[error("cannot parse integer: {0}")]
    BadInteger(#[from] ParseIntError),
    #[error("mate distance {0} is too large to fit into constraints")]
    MateTooLarge(i64),
}

fn parse_unbounded(tokens: &mut &[&UciToken], warn: &mut impl Sink<Error>) -> Option<RelScore> {
    match tok::next_warn(tokens, warn)?.as_str() {
        "cp" => {
            let value = tok::parse(tokens, warn)?;
            Some(RelScore::Cp(value))
        }
        "mate" => {
            let src: i64 = tok::parse(tokens, warn)?;
            let moves = src
                .abs()
                .try_into()
                .ok()
                .or_warn_with(Error::MateTooLarge(src), warn)?;
            Some(RelScore::Mate {
                moves,
                win: src > 0,
            })
        }
        tok => {
            warn.warn(Error::UnexpectedToken(tok.to_string()));
            None
        }
    }
}

pub fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Error>) -> Option<BoundedRelScore> {
    let score = parse_unbounded(tokens, warn)?;
    let bound = match tokens.first().map(|t| t.as_str()) {
        Some("lowerbound") => {
            *tokens = &tokens[1..];
            Bound::Lower
        }
        Some("upperbound") => {
            *tokens = &tokens[1..];
            Bound::Upper
        }
        _ => Bound::Exact,
    };
    Some(BoundedRelScore { score, bound })
}
