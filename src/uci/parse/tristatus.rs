use super::super::types::TriStatus;
use super::{prelude::*, tok};

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    UnexpectedEol(#[from] EolError),
    #[error("unexpected token: {0} (expected \"ok\", \"checking\" or \"error\")")]
    UnexpectedToken(String),
}

pub fn parse(tokens: &mut &[&Token], warn: &mut impl Sink<Error>) -> Option<TriStatus> {
    match tok::next_warn(tokens, warn)?.as_str() {
        "ok" => Some(TriStatus::Ok),
        "checking" => Some(TriStatus::Checking),
        "error" => Some(TriStatus::Error),
        tok => {
            warn.warn(Error::UnexpectedToken(tok.to_string()));
            None
        }
    }
}

pub fn fmt(src: &TriStatus, f: &mut impl PushTokens) {
    match src {
        TriStatus::Ok => f.push_kw("ok"),
        TriStatus::Checking => f.push_kw("checking"),
        TriStatus::Error => f.push_kw("error"),
    }
}
