use std::{error::Error, mem, num::ParseIntError, str::FromStr, time::Duration};

use owlchess::{
    board::RawFenParseError,
    moves::{uci, UciMove},
    RawBoard,
};

use thiserror::Error;

use super::{
    msg::{Command, GoLimits, Id, Info, Message, OptBody, Register},
    str::{self, OptComboVar, OptName, PushTokens, RegisterName, UciString, UciToken},
    types::{Permille, TriStatus},
};

use crate::score::{Bound, BoundedRelScore, RelScore};

use crate::warn::{OptionExt, ResultExt, Sink, SinkExt};

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
#[error("unexpected end of file")]
pub struct EolError;

fn try_split_at<'a, T: PartialEq<U>, U>(src: &'a [T], mid: U) -> (&'a [T], Option<&'a [T]>) {
    match src.iter().position(|v| *v == mid) {
        Some(pos) => (&src[..pos], Some(&src[pos + 1..])),
        None => (src, None),
    }
}

fn split_at<'a, T: PartialEq<U>, U, E: Error>(
    src: &'a [T],
    mid: U,
    error: E,
    warn: &mut impl Sink<E>,
) -> (&'a [T], &'a [T]) {
    let (l, r) = try_split_at(src, mid);
    (l, r.or_warn_with(error, warn).unwrap_or(&[]))
}

fn next_tok<'a>(tokens: &mut &[&'a UciToken]) -> Option<&'a UciToken> {
    let result;
    (result, *tokens) = tokens.split_first()?;
    Some(*result)
}

fn next_tok_warn<'a, E: From<EolError> + Error>(
    tokens: &mut &[&'a UciToken],
    warn: &mut impl Sink<E>,
) -> Option<&'a UciToken> {
    next_tok(tokens).or_warn_with(EolError.into(), warn)
}

fn parsed_tok<D, E, T>(tokens: &mut &[&UciToken], warn: &mut impl Sink<E>) -> Option<T>
where
    D: Error + Clone,
    E: From<D> + From<EolError> + Error,
    T: FromStr<Err = D>,
{
    parsed_tok_map(tokens, From::from, warn)
}

fn parsed_tok_map<D, E, F, T>(
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
    next_tok_warn(tokens, warn)?
        .as_str()
        .parse()
        .or_warn_map(func, warn)
        .ok()
}

fn expect_tok<E: From<EolError> + Error>(
    tokens: &mut &[&UciToken],
    expected: &str,
    on_mismatch: E,
    warn: &mut impl Sink<E>,
) -> Option<()> {
    if next_tok_warn(tokens, warn)? != expected {
        warn.warn(on_mismatch);
        return None;
    }
    Some(())
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("cannot parse move #{}: {}", pos + 1, error)]
pub struct MoveVecParseError {
    pos: usize,
    #[source]
    error: uci::RawParseError,
}

fn looks_like_move(tok: &UciToken) -> bool {
    let bytes = tok.as_bytes();
    matches!(bytes.len(), 4 | 5)
        && bytes[0].is_ascii_lowercase()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_lowercase()
        && bytes[3].is_ascii_digit()
}

fn parse_move_vec(
    tokens: &mut &[&UciToken],
    until_first_error: bool,
    warn: &mut impl Sink<MoveVecParseError>,
) -> Vec<UciMove> {
    let mut moves = Vec::new();
    while !tokens.is_empty() {
        let tok = tokens[0];
        if !looks_like_move(tok) {
            continue;
        }
        *tokens = &tokens[1..];
        match tok.parse::<UciMove>() {
            Ok(mv) => moves.push(mv),
            Err(error) => {
                warn.warn(MoveVecParseError {
                    pos: moves.len(),
                    error,
                });
                if until_first_error {
                    break;
                }
            }
        }
    }
    moves
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum CommandParseError {
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("extra token: {0}")]
    ExtraToken(String),
    #[error(transparent)]
    UnexpectedEol(#[from] EolError),
    #[error("no \"name\" in \"setoption\"")]
    SetOptionNoName,
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
    #[error("cannot parse searchmoves: {0}")]
    InvalidSearchMove(#[source] MoveVecParseError),
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

    while let Some(item) = next_tok(tokens).map(UciToken::as_str) {
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
                searchmoves = Some(parse_move_vec(
                    tokens,
                    false,
                    &mut warn.adapt_map(CommandParseError::InvalidSearchMove),
                ));
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
        GoLimits::Infinite
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
            match next_tok(tokens)?.as_str() {
                "uci" => return Some(Command::Uci),
                "debug" => {
                    return match next_tok_warn(tokens, warn)?.as_str() {
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
                    expect_tok(tokens, "name", CommandParseError::SetOptionNoName, warn)?;
                    let (name, value) = try_split_at(tokens, "value");
                    *tokens = &[];
                    let name = OptName::from_tokens(name)
                        .or_warn_map(CommandParseError::SetOptionBadName, warn)
                        .ok()?;
                    let value = value.map(UciString::from_tokens);
                    return Some(Command::SetOption { name, value });
                }
                "register" => {
                    return match next_tok_warn(tokens, warn)?.as_str() {
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
                    let startpos = match next_tok(&mut position).map(UciToken::as_str) {
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

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum TriStatusParseError {
    #[error(transparent)]
    UnexpectedEol(#[from] EolError),
    #[error("unexpected token: {0} (expected \"ok\", \"checking\" or \"error\")")]
    UnexpectedToken(String),
}

impl Parse for TriStatus {
    type Err = TriStatusParseError;

    fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Self::Err>) -> Option<Self> {
        match next_tok_warn(tokens, warn)?.as_str() {
            "ok" => Some(TriStatus::Ok),
            "checking" => Some(TriStatus::Checking),
            "error" => Some(TriStatus::Error),
            tok => {
                warn.warn(TriStatusParseError::UnexpectedToken(tok.to_string()));
                None
            }
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum RelScoreParseError {
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error(transparent)]
    UnexpectedEol(#[from] EolError),
    #[error("cannot parse integer: {0}")]
    BadInteger(#[from] ParseIntError),
    #[error("mate distance {0} is too large to fit into constraints")]
    MateTooLarge(i64),
}

impl Parse for RelScore {
    type Err = RelScoreParseError;

    fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Self::Err>) -> Option<Self> {
        match next_tok_warn(tokens, warn)?.as_str() {
            "cp" => {
                let value = parsed_tok(tokens, warn)?;
                Some(RelScore::Cp(value))
            }
            "mate" => {
                let src: i64 = parsed_tok(tokens, warn)?;
                let moves = src
                    .abs()
                    .try_into()
                    .ok()
                    .or_warn_with(RelScoreParseError::MateTooLarge(src), warn)?;
                Some(RelScore::Mate {
                    moves,
                    win: src > 0,
                })
            }
            tok => {
                warn.warn(RelScoreParseError::UnexpectedToken(tok.to_string()));
                None
            }
        }
    }
}

impl Parse for BoundedRelScore {
    type Err = RelScoreParseError;

    fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Self::Err>) -> Option<Self> {
        let score = RelScore::parse(tokens, warn)?;
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
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum InfoParseError {
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error(transparent)]
    UnexpectedEol(#[from] EolError),
    #[error("cannot parse integer: {0}")]
    BadInteger(#[from] ParseIntError),
    #[error("cannot parse move: {0}")]
    BadMove(#[from] uci::RawParseError),
    #[error("permille value {src_value} is larger than 1000, thus truncated")]
    PermilleTruncated { src_value: u64 },
    #[error("cannot parse move sequence: {0}")]
    BadMoveVec(#[from] MoveVecParseError),
    #[error("cannot parse score: {0}")]
    BadScore(#[from] RelScoreParseError),
}

fn make_permille(val: u64, warn: &mut impl Sink<InfoParseError>) -> Permille {
    if val >= 1000 {
        warn.warn(InfoParseError::PermilleTruncated { src_value: val });
    }
    Permille::new_truncated(val)
}

impl Parse for Info {
    type Err = InfoParseError;

    fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Self::Err>) -> Option<Self> {
        match next_tok_warn(tokens, warn)?.as_str() {
            "depth" => Some(Info::Depth(parsed_tok(tokens, warn)?)),
            "seldepth" => Some(Info::SelDepth(parsed_tok(tokens, warn)?)),
            "time" => Some(Info::Time(Duration::from_millis(parsed_tok(tokens, warn)?))),
            "nodes" => Some(Info::Nodes(parsed_tok(tokens, warn)?)),
            "pv" => Some(Info::Pv(parse_move_vec(tokens, true, &mut warn.adapt()))),
            "multipv" => Some(Info::MultiPv(parsed_tok(tokens, warn)?)),
            "score" => Some(Info::Score(BoundedRelScore::parse(
                tokens,
                &mut warn.adapt(),
            )?)),
            "currmove" => Some(Info::CurrMove(parsed_tok(tokens, warn)?)),
            "currmovenumber" => Some(Info::CurrMoveNumber(parsed_tok(tokens, warn)?)),
            "hashfull" => Some(Info::HashFull(make_permille(
                parsed_tok(tokens, warn)?,
                warn,
            ))),
            "nps" => Some(Info::Nps(parsed_tok(tokens, warn)?)),
            "tbhits" => Some(Info::TbHits(parsed_tok(tokens, warn)?)),
            "sbhits" => Some(Info::SbHits(parsed_tok(tokens, warn)?)),
            "cpuload" => Some(Info::CpuLoad(make_permille(
                parsed_tok(tokens, warn)?,
                warn,
            ))),
            "refutation" => Some(Info::Refutation(parse_move_vec(
                tokens,
                true,
                &mut warn.adapt(),
            ))),
            "currline" => {
                let cpu_num = parsed_tok(tokens, warn)?;
                let moves = parse_move_vec(tokens, true, &mut warn.adapt());
                Some(Info::CurrLine { cpu_num, moves })
            }
            tok => {
                warn.warn(InfoParseError::UnexpectedToken(tok.to_string()));
                None
            }
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum OptBodyParseError {
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

impl Parse for OptBody {
    type Err = OptBodyParseError;

    fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Self::Err>) -> Option<Self> {
        let result = (|| match next_tok_warn(tokens, warn)?.as_str() {
            "check" => {
                expect_tok(
                    tokens,
                    "default",
                    OptBodyParseError::ExpectedToken("default"),
                    warn,
                )?;
                let value = match next_tok_warn(tokens, warn)?.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => {
                        warn.warn(OptBodyParseError::ExpectedBool);
                        return None;
                    }
                };
                Some(OptBody::Check(value))
            }
            "spin" => {
                expect_tok(
                    tokens,
                    "default",
                    OptBodyParseError::ExpectedToken("default"),
                    warn,
                )?;
                let default = parsed_tok(tokens, warn)?;
                expect_tok(tokens, "min", OptBodyParseError::ExpectedToken("min"), warn)?;
                let min = parsed_tok(tokens, warn)?;
                expect_tok(tokens, "max", OptBodyParseError::ExpectedToken("max"), warn)?;
                let max = parsed_tok(tokens, warn)?;
                Some(OptBody::Spin { default, min, max })
            }
            "combo" => {
                expect_tok(
                    tokens,
                    "default",
                    OptBodyParseError::ExpectedToken("default"),
                    warn,
                )?;
                let mut iter = tokens.split(|&tok| tok == "var").fuse();
                *tokens = &[];
                let default = iter
                    .next()
                    .or_warn_with(OptBodyParseError::UnexpectedEol(EolError), warn)
                    .unwrap_or(&[]);
                let default = OptComboVar::from_tokens(default)
                    .or_warn_map(OptBodyParseError::BadComboDefaultVar, warn)
                    .ok()?;
                let vars: Vec<_> = iter
                    .enumerate()
                    .filter_map(|(pos, toks)| {
                        OptComboVar::from_tokens(toks)
                            .or_warn_map(
                                |error| OptBodyParseError::BadComboVar { pos, error },
                                warn,
                            )
                            .ok()
                    })
                    .collect();
                Some(OptBody::Combo { default, vars })
            }
            "button" => Some(OptBody::Button),
            "string" => {
                expect_tok(
                    tokens,
                    "default",
                    OptBodyParseError::ExpectedToken("default"),
                    warn,
                )?;
                Some(OptBody::String(UciString::from_tokens(mem::replace(
                    tokens,
                    &[],
                ))))
            }
            tok => {
                warn.warn(OptBodyParseError::UnknownType(tok.to_string()));
                None
            }
        })();
        if !tokens.is_empty() {
            warn.warn(OptBodyParseError::ExtraToken(tokens[0].to_string()));
        }
        result
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum MessageParseError {
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("extra token: {0}")]
    ExtraToken(String),
    #[error(transparent)]
    UnexpectedEol(#[from] EolError),
    #[error("invalid best move, assuming null move")]
    InvalidBestmove(#[source] uci::RawParseError),
    #[error("invalid ponder move")]
    InvalidPonder(#[source] uci::RawParseError),
    #[error("invalid copy protection status")]
    InvalidCopyProtection(#[source] TriStatusParseError),
    #[error("invalid registration status")]
    InvalidRegistration(#[source] TriStatusParseError),
    #[error("cannot parse info #{}: {}", pos + 1, error)]
    BadInfo {
        pos: usize,
        #[source]
        error: InfoParseError,
    },
    #[error("no \"name\" in \"option\"")]
    OptionNoName,
    #[error("no \"type\" in \"setoption\"")]
    OptionNoType,
    #[error("cannot convert option name: {0}")]
    OptionBadName(#[source] str::Error),
    #[error("invalid option body: {0}")]
    OptionBadBody(#[from] OptBodyParseError),
}

impl Parse for Message {
    type Err = MessageParseError;

    fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Self::Err>) -> Option<Self> {
        let result = (|| loop {
            match next_tok(tokens)?.as_str() {
                "id" => {
                    return match next_tok_warn(tokens, warn)?.as_str() {
                        "name" => {
                            let name = UciString::from_tokens(tokens);
                            *tokens = &[];
                            Some(Message::Id(Id::Name(name)))
                        }
                        "author" => {
                            let name = UciString::from_tokens(tokens);
                            *tokens = &[];
                            Some(Message::Id(Id::Author(name)))
                        }
                        tok => {
                            warn.warn(MessageParseError::UnexpectedToken(tok.to_string()));
                            None
                        }
                    }
                }
                "uciok" => return Some(Message::UciOk),
                "readyok" => return Some(Message::ReadyOk),
                "bestmove" => {
                    let bestmove = next_tok_warn(tokens, warn)?;
                    let bestmove = bestmove
                        .parse()
                        .or_warn_map(MessageParseError::InvalidBestmove, warn)
                        .unwrap_or(UciMove::Null);
                    let ponder = (|| {
                        let tok = next_tok(tokens)?;
                        if tok != "ponder" {
                            warn.warn(MessageParseError::UnexpectedToken(tok.to_string()));
                            return None;
                        }
                        let ponder = next_tok_warn(tokens, warn)?;
                        UciMove::from_str(ponder)
                            .or_warn_map(MessageParseError::InvalidPonder, warn)
                            .ok()
                    })();
                    return Some(Message::BestMove { bestmove, ponder });
                }
                "copyprotection" => {
                    let status = TriStatus::parse(
                        tokens,
                        &mut warn.adapt_map(MessageParseError::InvalidCopyProtection),
                    )?;
                    return Some(Message::CopyProtection(status));
                }
                "registration" => {
                    let status = TriStatus::parse(
                        tokens,
                        &mut warn.adapt_map(MessageParseError::InvalidRegistration),
                    )?;
                    return Some(Message::Registration(status));
                }
                "info" => {
                    let mut info = Vec::new();
                    let mut string = None;
                    while !tokens.is_empty() {
                        if tokens[0] == "string" {
                            string = Some(UciString::from_tokens(&tokens[1..]));
                            *tokens = &[];
                            break;
                        }
                        let pos = info.len();
                        if let Some(inf) = Info::parse(
                            tokens,
                            &mut warn.adapt_map(|error| MessageParseError::BadInfo { pos, error }),
                        ) {
                            info.push(inf);
                        }
                    }
                    return Some(Message::Info { info, string });
                }
                "option" => {
                    expect_tok(tokens, "name", MessageParseError::OptionNoName, warn)?;
                    let (name, mut body) =
                        split_at(tokens, "type", MessageParseError::OptionNoType, warn);
                    *tokens = &[];
                    let name = OptName::from_tokens(name)
                        .or_warn_map(MessageParseError::OptionBadName, warn)
                        .ok()?;
                    let body = OptBody::parse(&mut body, &mut warn.adapt())?;
                    return Some(Message::Option { name, body });
                }
                tok => warn.warn(MessageParseError::UnexpectedToken(tok.to_string())),
            }
        })();
        if !tokens.is_empty() {
            warn.warn(MessageParseError::UnexpectedToken(tokens[0].to_string()));
        }
        result
    }
}
