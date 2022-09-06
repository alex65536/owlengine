use owlchess::board::{RawBoard, RawFenParseError};

use super::super::{
    msg::{Command, GoLimits, Register},
    str::{OptName, RegisterName},
};
use super::{movevec, prelude::*, tok};

#[derive(Clone, Debug, Error, Eq, PartialEq)]
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

fn parse_go(tokens: &mut &[&UciToken], warn: &mut impl Sink<Error>) -> Command {
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

    while let Some(item) = tok::next(tokens).map(UciToken::as_str) {
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

pub fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Error>) -> Option<Command> {
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
                    .or_warn_map(Error::SetOptionBadName, warn)
                    .ok()?;
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
                            .or_warn_map(Error::RegisterBadName, warn)
                            .ok()?;
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
                let startpos = match tok::next(&mut position).map(UciToken::as_str) {
                    Some("startpos") => {
                        if !position.is_empty() {
                            warn.warn(Error::ExtraToken(position[0].to_string()));
                        }
                        RawBoard::initial()
                    }
                    Some("fen") => RawBoard::from_fen(&position.join(" ")).or_warn(warn).ok()?,
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
                    .or_warn(warn)
                    .ok()?;
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
        Command::Uci => f.do_tok("uci"),
        Command::Debug(val) => {
            f.do_tok("debug");
            f.do_tok(if *val { "on" } else { "off" });
        }
        Command::IsReady => f.do_tok("isready"),
        Command::SetOption { name, value } => {
            f.do_tok("setoption");
            f.do_tok("name");
            f.push_str(name.as_ref());
            if let Some(value) = value {
                f.do_tok("value");
                f.push_str(value);
            }
        }
        Command::Register(reg) => {
            f.do_tok("register");
            match reg {
                Register::Later => f.do_tok("later"),
                Register::Now { name, code } => {
                    f.do_tok("name");
                    f.push_str(name);
                    f.do_tok("code");
                    f.push_str(code);
                }
            }
        }
        Command::UciNewGame => f.do_tok("ucinewgame"),
        Command::Position { startpos, moves } => {
            f.do_tok("position");
            if startpos == &RawBoard::initial() {
                f.do_tok("startpos");
            } else {
                f.do_tok("fen");
                f.push_str(&UciString::from(&startpos.to_string()));
            }
            f.do_tok("moves");
            for mv in moves {
                f.do_tok(&mv.to_string());
            }
        }
        Command::Go {
            searchmoves,
            ponder,
            limits,
        } => {
            f.do_tok("go");
            if let Some(searchmoves) = searchmoves {
                f.do_tok("searchmoves");
                movevec::fmt(searchmoves, f);
            }
            if ponder.is_some() {
                f.do_tok("ponder");
            }
            match limits {
                GoLimits::Infinite => f.do_tok("infinite"),
                GoLimits::Clock {
                    wtime,
                    btime,
                    winc,
                    binc,
                    movestogo,
                } => {
                    f.do_tok("wtime");
                    f.do_tok(&wtime.as_millis().to_string());
                    f.do_tok("btime");
                    f.do_tok(&btime.as_millis().to_string());
                    if winc != &Duration::ZERO {
                        f.do_tok("winc");
                        f.do_tok(&winc.as_millis().to_string());
                    }
                    if binc != &Duration::ZERO {
                        f.do_tok("binc");
                        f.do_tok(&binc.as_millis().to_string());
                    }
                    if let Some(movestogo) = movestogo {
                        f.do_tok("movestogo");
                        f.do_tok(&movestogo.to_string());
                    }
                }
                GoLimits::Mate(value) => {
                    f.do_tok("mate");
                    f.do_tok(&value.to_string());
                }
                GoLimits::Limits {
                    depth,
                    nodes,
                    movetime,
                } => {
                    if depth.is_none() && nodes.is_none() && movetime.is_none() {
                        f.do_tok("infinite");
                    }
                    if let Some(depth) = depth {
                        f.do_tok("depth");
                        f.do_tok(&depth.to_string());
                    }
                    if let Some(nodes) = nodes {
                        f.do_tok("nodes");
                        f.do_tok(&nodes.to_string());
                    }
                    if let Some(movetime) = movetime {
                        f.do_tok("movetime");
                        f.do_tok(&movetime.as_millis().to_string());
                    }
                }
            }
        }
        Command::Stop => f.do_tok("stop"),
        Command::PonderHit => f.do_tok("ponderhit"),
        Command::Quit => f.do_tok("quit"),
    }
}
