mod command;
mod info;
mod message;
mod movevec;
mod optbody;
mod score;
mod tok;
mod tristatus;

mod prelude {
    pub use super::super::str::{Error as StrError, PushTokens, UciString, UciToken};
    pub use super::{EolError, tok::PushTokensExt};
    pub use crate::warn::{OptionExt, ResultExt, Sink, SinkExt};
    pub use owlchess::moves::{uci, UciMove};
    pub use std::{num::ParseIntError, time::Duration};
    pub use thiserror::Error;
}

use std::error::Error;

use thiserror::Error;

use crate::warn::Sink;

use super::{
    msg::{Command, Message},
    str::{self, PushTokens, UciToken, UciString},
};

pub trait Parse {
    type Err: Error;

    fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Self::Err>) -> Option<Self>
    where
        Self: Sized;

    #[inline]
    fn parse_line(line: &str, warn: &mut impl Sink<Self::Err>) -> Option<Self>
    where
        Self: Sized
    {
        let tokens: Vec<_> = str::tokenize(line).collect();
        Self::parse(&mut &tokens[..], warn)
    }
}

pub trait Fmt {
    fn fmt(&self, f: &mut impl PushTokens);

    #[inline]
    fn fmt_line(&self) -> String {
        let mut res = UciString::default();
        self.fmt(&mut res);
        res.into()
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("unexpected end of file")]
pub struct EolError;

pub use command::Error as CommandError;
pub use info::Error as InfoError;
pub use message::Error as MessageError;
pub use movevec::Error as MoveVecError;
pub use optbody::Error as OptBodyError;
pub use score::Error as ScoreError;
pub use tristatus::Error as TriStatusError;

impl Parse for Command {
    type Err = command::Error;

    fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Self::Err>) -> Option<Self> {
        command::parse(tokens, warn)
    }
}

impl Fmt for Command {
    fn fmt(&self, f: &mut impl PushTokens) {
        command::fmt(self, f)
    }
}

impl Parse for Message {
    type Err = message::Error;

    fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Self::Err>) -> Option<Self> {
        message::parse(tokens, warn)
    }
}

impl Fmt for Message {
    fn fmt(&self, f: &mut impl PushTokens) {
        message::fmt(self, f)
    }
}
