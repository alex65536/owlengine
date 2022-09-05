use std::{error::Error, str::FromStr};

use owlchess::{
    board::RawFenParseError,
    moves::{uci, UciMove},
    RawBoard,
};

use thiserror::Error;

use super::{
    msg::{Command, Register},
    str::{self, OptName, PushTokens, RegisterName, UciString, UciToken},
};

use crate::warn::{OptionExt, ResultExt, Sink};

pub trait Parse {
    type Err: Error;

    fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Self::Err>) -> Option<Self>
    where
        Self: Sized;
}

pub trait Fmt {
    fn fmt(&self, f: &mut impl PushTokens);
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum CommandParseError {
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("extra token: {0}")]
    ExtraToken(String),
    #[error("unexpected end of line")]
    UnexpectedEol,
    #[error("no \"name\" in \"setoption\"")]
    SetOptionNoName,
    #[error("no \"value\" in \"setoption\"")]
    SetOptionNoValue,
    #[error("cannot convert option name: {0}")]
    SetOptionBadName(#[source] str::Error),
    #[error("no \"code\" in \"register\"")]
    RegisterNoCode,
    #[error("cannot convert register name: {0}")]
    RegisterBadName(#[source] str::Error),
    #[error("no \"moves\" in position")]
    PositionNoMoves,
    #[error("no position specified, assuming \"startpos\"")]
    NoPosition,
    #[error("cannot parse FEN")]
    InvalidFen(#[from] RawFenParseError),
    #[error("cannot parse move #{}: {}", pos + 1, error)]
    InvalidMove {
        pos: usize,
        #[source]
        error: uci::RawParseError,
    },
}

impl Parse for Command {
    type Err = CommandParseError;

    fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Self::Err>) -> Option<Self> {
        let result = (|| loop {
            match next_item(tokens)?.as_str() {
                "uci" => return Some(Command::Uci),
                "debug" => {
                    return match next_item(tokens)
                        .or_warn_with(CommandParseError::UnexpectedEol, warn)?
                        .as_str()
                    {
                        "on" => Some(Command::Debug(true)),
                        "off" => Some(Command::Debug(false)),
                        tok => {
                            warn.warn(CommandParseError::UnexpectedToken(tok.to_string()));
                            None
                        }
                    }
                }
                "isready" => return Some(Command::IsReady),
                "setoption" => {
                    let name_kw =
                        next_item(tokens).or_warn_with(CommandParseError::UnexpectedEol, warn)?;
                    if name_kw != "name" {
                        warn.warn(CommandParseError::SetOptionNoName);
                        return None;
                    }
                    let (name, value) =
                        split_at(tokens, "value", CommandParseError::SetOptionNoValue, warn);
                    *tokens = &[];
                    let name = OptName::from_tokens(name)
                        .or_warn_map(CommandParseError::SetOptionBadName, warn)
                        .ok()?;
                    let value = UciString::from_tokens(value);
                    return Some(Command::SetOption { name, value });
                }
                "register" => {
                    return match next_item(tokens)
                        .or_warn_with(CommandParseError::UnexpectedEol, warn)?
                        .as_str()
                    {
                        "later" => Some(Command::Register(Register::Later)),
                        "name" => {
                            let (name, code) =
                                split_at(tokens, "code", CommandParseError::RegisterNoCode, warn);
                            *tokens = &[];
                            let name = RegisterName::from_tokens(name)
                                .or_warn_map(CommandParseError::RegisterBadName, warn)
                                .ok()?;
                            let code = UciString::from_tokens(code);
                            Some(Command::Register(Register::Now { name, code }))
                        }
                        tok => {
                            warn.warn(CommandParseError::UnexpectedToken(tok.to_string()));
                            None
                        }
                    }
                }
                "ucinewgame" => return Some(Command::UciNewGame),
                "position" => {
                    let (mut position, moves) =
                        split_at(tokens, "moves", CommandParseError::PositionNoMoves, warn);
                    *tokens = &[];
                    let startpos = match next_item(&mut position).map(UciToken::as_str) {
                        Some("startpos") => {
                            if !position.is_empty() {
                                warn.warn(CommandParseError::ExtraToken(position[0].to_string()));
                            }
                            RawBoard::initial()
                        }
                        Some("fen") => {
                            RawBoard::from_fen(&position.join(" ")).or_warn(warn).ok()?
                        }
                        Some(tok) => {
                            warn.warn(CommandParseError::UnexpectedToken(tok.to_string()));
                            return None;
                        }
                        None => {
                            warn.warn(CommandParseError::NoPosition);
                            RawBoard::initial()
                        }
                    };
                    let moves = moves
                        .iter()
                        .enumerate()
                        .map(|(pos, tok)| {
                            UciMove::from_str(*tok)
                                .map_err(|error| CommandParseError::InvalidMove { pos, error })
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .or_warn(warn)
                        .ok()?;
                    return Some(Command::Position { startpos, moves });
                }
                "go" => todo!(),
                "stop" => return Some(Command::Stop),
                "ponderhit" => return Some(Command::PonderHit),
                "quit" => return Some(Command::Quit),
                tok => warn.warn(CommandParseError::UnexpectedToken(tok.to_string())),
            }
        })();
        if !tokens.is_empty() {
            warn.warn(CommandParseError::ExtraToken(tokens[0].to_string()));
        }
        result
    }
}

fn split_at<'a, T, U, E>(
    src: &'a [T],
    mid: U,
    error: E,
    warn: &mut impl Sink<E>,
) -> (&'a [T], &'a [T])
where
    E: Error,
    T: PartialEq<U>,
{
    match src.iter().position(|v| *v == mid).or_warn_with(error, warn) {
        Some(pos) => (&src[..pos], &src[pos + 1..]),
        None => (src, &[]),
    }
}

fn next_item<T: Copy>(src: &mut &[T]) -> Option<T> {
    let result;
    (result, *src) = src.split_first()?;
    Some(*result)
}
