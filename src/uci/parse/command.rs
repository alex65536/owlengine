use owlchess::board::{RawBoard, RawFenParseError};

use super::super::{
    msg::{Command, GoLimits, Register},
    str::{OptName, RegisterName},
};
use super::{movevec, prelude::*, tok};

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
    #[error("duplicate item in \"go\": \"{0}\"")]
    GoDuplicate(&'static str),
    #[error("cannot parse searchmoves: {0}")]
    InvalidSearchMove(#[source] movevec::Error),
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

fn parse_go(tokens: &mut &[&Token], warn: &mut impl Warn<Error>) -> Command {
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

    while let Some(item) = tok::next(tokens).map(Token::as_str) {
        macro_rules! parse_int {
            ($ident:ident) => {{
                if $ident.is_some() {
                    warn.warn(Error::GoDuplicate(stringify!($ident)));
                }
                if let Some(value) = tok::parse_map(
                    tokens,
                    |error| Error::GoInvalidIntSub {
                        name: stringify!($ident),
                        error,
                    },
                    warn,
                ) {
                    $ident = Some(value);
                }
            }};
        }

        match item {
            "searchmoves" => {
                if ponder.is_some() {
                    warn.warn(Error::GoDuplicate("searchmoves"));
                }
                searchmoves = Some(movevec::parse(
                    tokens,
                    false,
                    &mut warn.adapt_map(Error::InvalidSearchMove),
                ));
            }
            "ponder" => {
                if ponder.is_some() {
                    warn.warn(Error::GoDuplicate("ponder"));
                }
                ponder = Some(());
            }
            "infinite" => {
                if infinite.is_some() {
                    warn.warn(Error::GoDuplicate("infinite"));
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
            tok => warn.warn(Error::UnexpectedToken(tok.to_string())),
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
        warn.warn(Error::GoNoLimits);
        GoLimits::Infinite
    })();

    macro_rules! verify_taken {
        ($($item:ident),*) => {
            $(
                if $item.is_some() {
                    warn.warn(Error::GoConflict(stringify!($item)));
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
                let name = OptName::from_tokens(name)
                    .or_warn_map(Error::SetOptionBadName, warn)?;
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
            "go" => return Some(parse_go(tokens, warn)),
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
        Command::Go {
            searchmoves,
            ponder,
            limits,
        } => {
            f.push_kw("go");
            if let Some(searchmoves) = searchmoves {
                f.push_kw("searchmoves");
                movevec::fmt(searchmoves, f);
            }
            if ponder.is_some() {
                f.push_kw("ponder");
            }
            match limits {
                GoLimits::Infinite => f.push_kw("infinite"),
                GoLimits::Clock {
                    wtime,
                    btime,
                    winc,
                    binc,
                    movestogo,
                } => {
                    f.push_tag("wtime", &wtime.as_millis());
                    f.push_tag("btime", &btime.as_millis());
                    if winc != &Duration::ZERO {
                        f.push_tag("winc", &winc.as_millis());
                    }
                    if binc != &Duration::ZERO {
                        f.push_tag("binc", &binc.as_millis());
                    }
                    if let Some(movestogo) = movestogo {
                        f.push_tag("movestogo", movestogo);
                    }
                }
                GoLimits::Mate(value) => f.push_tag("mate", value),
                GoLimits::Limits {
                    depth,
                    nodes,
                    movetime,
                } => {
                    if depth.is_none() && nodes.is_none() && movetime.is_none() {
                        f.push_kw("infinite");
                    }
                    if let Some(depth) = depth {
                        f.push_tag("depth", depth);
                    }
                    if let Some(nodes) = nodes {
                        f.push_tag("nodes", nodes);
                    }
                    if let Some(movetime) = movetime {
                        f.push_tag("movetime", &movetime.as_millis());
                    }
                }
            }
        }
        Command::Stop => f.push_kw("stop"),
        Command::PonderHit => f.push_kw("ponderhit"),
        Command::Quit => f.push_kw("quit"),
    }
}
