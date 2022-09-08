use owlchess::board::{RawBoard, RawFenParseError};

use super::super::{
    msg::{Command, Register},
    str::{OptName, RegisterName},
};
use super::{go, prelude::*, tok};

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("extra token: {0}")]
    ExtraToken(String),
    #[error(transparent)]
    UnexpectedEol(#[from] EolError),
    #[error("no \"name\" in \"setoption\"")]
    SetOptionNoName,
    #[error("cannot convert option name: {0}")]
    SetOptionBadName(#[source] StrError),
    #[error("no \"code\" in \"register\"")]
    RegisterNoCode,
    #[error("cannot convert register name: {0}")]
    RegisterBadName(#[source] StrError),
    #[error("no \"moves\" in \"position\"")]
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
    #[error("invalid \"go\" options: {0}")]
    InvalidGo(#[from] go::Error),
}

pub fn parse(tokens: &mut &[&Token], warn: &mut impl Warn<Error>) -> Option<Command> {
    let result = (|| loop {
        match tok::next(tokens)?.as_str() {
            "uci" => return Some(Command::Uci),
            "debug" => {
                return match tok::next_warn(tokens, warn)?.as_str() {
                    "on" => Some(Command::Debug(true)),
                    "off" => Some(Command::Debug(false)),
                    tok => {
                        warn.warn(Error::UnexpectedToken(tok.to_string()));
                        None
                    }
                }
            }
            "isready" => return Some(Command::IsReady),
            "setoption" => {
                tok::expect(tokens, "name", Error::SetOptionNoName, warn)?;
                let (name, value) = tok::try_split(tokens, "value");
                *tokens = &[];
                let name = OptName::from_tokens(name).or_warn_map(Error::SetOptionBadName, warn)?;
                let value = value.map(UciString::from_tokens);
                return Some(Command::SetOption { name, value });
            }
            "register" => {
                return match tok::next_warn(tokens, warn)?.as_str() {
                    "later" => Some(Command::Register(Register::Later)),
                    "name" => {
                        let (name, code) = tok::split(tokens, "code", Error::RegisterNoCode, warn);
                        *tokens = &[];
                        let name = RegisterName::from_tokens(name)
                            .or_warn_map(Error::RegisterBadName, warn)?;
                        let code = UciString::from_tokens(code);
                        Some(Command::Register(Register::Now { name, code }))
                    }
                    tok => {
                        warn.warn(Error::UnexpectedToken(tok.to_string()));
                        None
                    }
                }
            }
            "ucinewgame" => return Some(Command::UciNewGame),
            "position" => {
                let (mut position, moves) =
                    tok::split(tokens, "moves", Error::PositionNoMoves, warn);
                *tokens = &[];
                let startpos = match tok::next(&mut position).map(Token::as_str) {
                    Some("startpos") => {
                        if !position.is_empty() {
                            warn.warn(Error::ExtraToken(position[0].to_string()));
                        }
                        RawBoard::initial()
                    }
                    Some("fen") => RawBoard::from_fen(&position.join(" ")).or_warn(warn)?,
                    Some(tok) => {
                        warn.warn(Error::UnexpectedToken(tok.to_string()));
                        return None;
                    }
                    None => {
                        warn.warn(Error::NoPosition);
                        RawBoard::initial()
                    }
                };
                let moves = moves
                    .iter()
                    .enumerate()
                    .map(|(pos, tok)| {
                        tok.parse()
                            .map_err(|error| Error::InvalidMove { pos, error })
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .or_warn(warn)?;
                return Some(Command::Position { startpos, moves });
            }
            "go" => return Some(Command::Go(go::parse(tokens, &mut warn.adapt()))),
            "stop" => return Some(Command::Stop),
            "ponderhit" => return Some(Command::PonderHit),
            "quit" => return Some(Command::Quit),
            tok => warn.warn(Error::UnexpectedToken(tok.to_string())),
        }
    })();
    if !tokens.is_empty() {
        warn.warn(Error::ExtraToken(tokens[0].to_string()));
    }
    result
}

pub fn fmt(src: &Command, f: &mut impl PushTokens) {
    match src {
        Command::Uci => f.push_kw("uci"),
        Command::Debug(val) => {
            f.push_kw("debug");
            f.push_kw(if *val { "on" } else { "off" });
        }
        Command::IsReady => f.push_kw("isready"),
        Command::SetOption { name, value } => {
            f.push_kw("setoption");
            f.push_tag_many("name", name);
            if let Some(value) = value {
                f.push_tag_many("value", value);
            }
        }
        Command::Register(reg) => {
            f.push_kw("register");
            match reg {
                Register::Later => f.push_kw("later"),
                Register::Now { name, code } => {
                    f.push_tag_many("name", name);
                    f.push_tag_many("code", code);
                }
            }
        }
        Command::UciNewGame => f.push_kw("ucinewgame"),
        Command::Position { startpos, moves } => {
            f.push_kw("position");
            if startpos == &RawBoard::initial() {
                f.push_kw("startpos");
            } else {
                f.push_tag_many("fen", startpos);
            }
            f.push_kw("moves");
            for mv in moves {
                f.push_fmt(mv);
            }
        }
        Command::Go(go) => {
            f.push_kw("go");
            go::fmt(go, f);
        }
        Command::Stop => f.push_kw("stop"),
        Command::PonderHit => f.push_kw("ponderhit"),
        Command::Quit => f.push_kw("quit"),
    }
}
