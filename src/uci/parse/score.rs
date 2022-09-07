use super::{prelude::*, tok};
use crate::score::{Bound, BoundedRelScore, RelScore};

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

fn parse_unbounded(tokens: &mut &[&Token], warn: &mut impl Sink<Error>) -> Option<RelScore> {
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

pub fn parse(tokens: &mut &[&Token], warn: &mut impl Sink<Error>) -> Option<BoundedRelScore> {
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

fn fmt_unbounded(src: &RelScore, f: &mut impl PushTokens) {
    match src {
        RelScore::Cp(val) => {
            f.do_tok("cp");
            f.do_tok(&val.to_string());
        }
        RelScore::Mate { moves, win } => {
            let mut moves = *moves as i64;
            if !win {
                moves = -moves;
            }
            f.do_tok("mate");
            f.do_tok(&moves.to_string());
        }
    }
}

pub fn fmt(src: &BoundedRelScore, f: &mut impl PushTokens) {
    fmt_unbounded(&src.score, f);
    match src.bound {
        Bound::Lower => f.do_tok("lowerbound"),
        Bound::Upper => f.do_tok("upperbound"),
        Bound::Exact => {}
    }
}
