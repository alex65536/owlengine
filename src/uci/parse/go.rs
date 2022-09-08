use super::super::msg::Go;
use super::{movevec, prelude::*, tok};

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error(transparent)]
    UnexpectedEol(#[from] EolError),
    #[error("duplicate item \"{0}\"")]
    Duplicate(&'static str),
    #[error("cannot parse searchmoves: {0}")]
    InvalidSearchMove(#[source] movevec::Error),
    #[error("cannot parse integer for \"{name}\": {error}")]
    InvalidIntSub {
        name: &'static str,
        #[source]
        error: ParseIntError,
    },
}

pub fn parse(tokens: &mut &[&Token], warn: &mut impl Warn<Error>) -> Go {
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
                    warn.warn(Error::Duplicate(stringify!($ident)));
                }
                if let Some(value) = tok::parse_map(
                    tokens,
                    |error| Error::InvalidIntSub {
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
                    warn.warn(Error::Duplicate("searchmoves"));
                }
                searchmoves = Some(movevec::parse(
                    tokens,
                    false,
                    &mut warn.adapt_map(Error::InvalidSearchMove),
                ));
            }
            "ponder" => {
                if ponder.is_some() {
                    warn.warn(Error::Duplicate("ponder"));
                }
                ponder = Some(());
            }
            "infinite" => {
                if infinite.is_some() {
                    warn.warn(Error::Duplicate("infinite"));
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

    Go {
        searchmoves,
        ponder,
        infinite,
        wtime: wtime.map(Duration::from_millis),
        winc: winc.map(Duration::from_millis),
        btime: btime.map(Duration::from_millis),
        binc: binc.map(Duration::from_millis),
        movestogo,
        mate,
        depth,
        nodes,
        movetime: movetime.map(Duration::from_millis),
    }
}

pub fn fmt(src: &Go, f: &mut impl PushTokens) {
    if let Some(val) = &src.searchmoves {
        f.push_kw("searchmoves");
        movevec::fmt(val, f);
    }
    if src.ponder.is_some() {
        f.push_kw("ponder");
    }
    if src.infinite.is_some() {
        f.push_kw("infinite");
    }
    if let Some(val) = &src.wtime {
        f.push_tag("wtime", &val.as_millis());
    }
    if let Some(val) = &src.btime {
        f.push_tag("btime", &val.as_millis());
    }
    if let Some(val) = &src.winc {
        f.push_tag("winc", &val.as_millis());
    }
    if let Some(val) = &src.binc {
        f.push_tag("binc", &val.as_millis());
    }
    if let Some(val) = &src.movestogo {
        f.push_tag("movestogo", val);
    }
    if let Some(val) = &src.mate {
        f.push_tag("mate", val);
    }
    if let Some(val) = &src.depth {
        f.push_tag("depth", val);
    }
    if let Some(val) = &src.nodes {
        f.push_tag("nodes", val);
    }
    if let Some(val) = &src.movetime {
        f.push_tag("movetime", &val.as_millis());
    }
}
