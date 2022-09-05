use std::{error::Error, num::ParseIntError, str::FromStr, time::Duration};

use owlchess::{
    board::RawFenParseError,
    moves::{uci, UciMove},
    RawBoard,
};

use thiserror::Error;

use super::{
    msg::{Command, GoLimits, Register},
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
    #[error("duplicate item in \"go\": \"{0}\"")]
    GoDuplicate(&'static str),
    #[error("cannot parse move \"{token}\": {error}")]
    InvalidSearchMove {
        token: String,
        #[source]
        error: uci::RawParseError,
    },
    #[error("cannot parse integer \"{name}\" in \"go\": {error}")]
    GoInvalidIntSub {
        name: &'static str,
        #[source]
        error: ParseIntError,
    },
    #[error("go item \"{0}\" ignored because of conflict with other items")]
    GoConflict(&'static str),
    #[error("no useful limits specified, considering \"go infinite\"")]
    GoNoLimits,
}

fn parse_go(tokens: &mut &[&UciToken], warn: &mut impl Sink<CommandParseError>) -> Command {
    let mut searchmoves = None;
    let mut ponder = None;
    let mut infinite = None;
    let mut wtime = None;
    let mut btime = None;
    let mut winc = None;
    let mut binc = None;
    let mut movestogo = None;
    let mut mate = None;
    let mut depth = None;
    let mut nodes = None;
    let mut movetime = None;

    while let Some(item) = next_item(tokens).map(UciToken::as_str) {
        macro_rules! parse_int {
            ($ident:ident) => {{
                if $ident.is_some() {
                    warn.warn(CommandParseError::GoDuplicate(stringify!($ident)));
                }
                match item.parse() {
                    Ok(value) => $ident = Some(value),
                    Err(err) => warn.warn(CommandParseError::GoInvalidIntSub {
                        name: stringify!($ident),
                        error: err,
                    }),
                }
            }};
        }

        match item {
            "searchmoves" => {
                if ponder.is_some() {
                    warn.warn(CommandParseError::GoDuplicate("searchmoves"));
                }
                let mut moves = Vec::new();
                while !tokens.is_empty() {
                    let tok = tokens[0];
                    let bytes = tok.as_bytes();
                    // Guess whether the next token is an UCI move.
                    if !(matches!(bytes.len(), 4 | 5)
                        && bytes[0].is_ascii_lowercase()
                        && bytes[1].is_ascii_digit()
                        && bytes[2].is_ascii_lowercase()
                        && bytes[3].is_ascii_digit())
                    {
                        continue;
                    }
                    // Heuristics passed, treat this token as an UCI move.
                    *tokens = &tokens[1..];
                    match tok.parse::<UciMove>() {
                        Ok(mv) => moves.push(mv),
                        Err(error) => warn.warn(CommandParseError::InvalidSearchMove {
                            token: tok.to_string(),
                            error,
                        }),
                    }
                }
                searchmoves = Some(moves);
            }
            "ponder" => {
                if ponder.is_some() {
                    warn.warn(CommandParseError::GoDuplicate("ponder"));
                }
                ponder = Some(());
            }
            "infinite" => {
                if infinite.is_some() {
                    warn.warn(CommandParseError::GoDuplicate("infinite"));
                }
                infinite = Some(());
            }
            "wtime" => parse_int!(wtime),
            "btime" => parse_int!(btime),
            "winc" => parse_int!(winc),
            "binc" => parse_int!(binc),
            "movestogo" => parse_int!(movestogo),
            "mate" => parse_int!(mate),
            "depth" => parse_int!(depth),
            "nodes" => parse_int!(nodes),
            "movetime" => parse_int!(movetime),
            tok => warn.warn(CommandParseError::UnexpectedToken(tok.to_string())),
        }
    }

    let limits = (|| {
        if infinite.is_some() {
            infinite = None;
            return GoLimits::Infinite;
        }
        if mate.is_some() {
            return GoLimits::Mate(mate.take().unwrap());
        }
        if wtime.is_some() && btime.is_some() {
            return GoLimits::Clock {
                wtime: Duration::from_millis(wtime.take().unwrap()),
                btime: Duration::from_millis(btime.take().unwrap()),
                winc: Duration::from_millis(winc.take().unwrap_or(0)),
                binc: Duration::from_millis(binc.take().unwrap_or(0)),
                movestogo: movestogo.take(),
            };
        }
        if depth.is_some() || nodes.is_some() || movetime.is_some() {
            return GoLimits::Limits {
                depth: depth.take(),
                nodes: nodes.take(),
                movetime: movetime.take().map(Duration::from_millis),
            };
        }
        warn.warn(CommandParseError::GoNoLimits);
        return GoLimits::Infinite;
    })();

    macro_rules! verify_taken {
        ($($item:ident),*) => {
            $(
                if $item.is_some() {
                    warn.warn(CommandParseError::GoConflict(stringify!($item)));
                }
            )*
        }
    }

    verify_taken!(infinite, wtime, btime, winc, binc, movestogo, mate, depth, nodes, movetime);

    Command::Go {
        searchmoves,
        ponder,
        limits,
    }
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
                "go" => return Some(parse_go(tokens, warn)),
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
