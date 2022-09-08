mod command;
mod go;
mod info;
mod message;
mod movevec;
mod optbody;
mod score;
mod tok;
mod tristatus;

mod prelude {
    pub use super::super::{
        str::{Error as StrError, UciString},
        token::{PushTokens, Token},
    };
    pub use super::{tok::PushTokensExt, EolError};
    pub use owlchess::moves::{uci, UciMove};
    pub use std::{num::ParseIntError, time::Duration};
    pub use thiserror::Error;
    pub use wurm::prelude::*;
}

use std::error::Error;

use thiserror::Error;

use wurm::Warn;

use super::{
    msg::{Command, Message},
    str::UciString,
    token::{self, PushTokens, Token},
};

pub trait Parse {
    type Err: Error;

    fn parse(tokens: &mut &[&Token], warn: &mut impl Warn<Self::Err>) -> Option<Self>
    where
        Self: Sized;

    #[inline]
    fn parse_line(line: &str, warn: &mut impl Warn<Self::Err>) -> Option<Self>
    where
        Self: Sized,
    {
        let tokens: Vec<_> = token::tokenize(line).collect();
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
#[error("unexpected end of line")]
pub struct EolError;

pub use command::Error as CommandError;
pub use go::Error as GoError;
pub use info::Error as InfoError;
pub use message::Error as MessageError;
pub use movevec::Error as MoveVecError;
pub use optbody::Error as OptBodyError;
pub use score::Error as ScoreError;
pub use tristatus::Error as TriStatusError;

impl Parse for Command {
    type Err = command::Error;

    fn parse(tokens: &mut &[&Token], warn: &mut impl Warn<Self::Err>) -> Option<Self> {
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

    fn parse(tokens: &mut &[&Token], warn: &mut impl Warn<Self::Err>) -> Option<Self> {
        message::parse(tokens, warn)
    }
}

impl Fmt for Message {
    fn fmt(&self, f: &mut impl PushTokens) {
        message::fmt(self, f)
    }
}
