use super::super::{msg::Info, types::Permille};
use super::{movevec, prelude::*, score, tok};

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
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
    BadMoveVec(#[from] movevec::Error),
    #[error("cannot parse score: {0}")]
    BadScore(#[from] score::Error),
}

fn make_permille(val: u64, warn: &mut impl Warn<Error>) -> Permille {
    if val >= 1000 {
        warn.warn(Error::PermilleTruncated { src_value: val });
    }
    Permille::new_truncated(val)
}

pub fn parse(tokens: &mut &[&Token], warn: &mut impl Warn<Error>) -> Option<Info> {
    match tok::next_warn(tokens, warn)?.as_str() {
        "depth" => Some(Info::Depth(tok::parse(tokens, warn)?)),
        "seldepth" => Some(Info::SelDepth(tok::parse(tokens, warn)?)),
        "time" => Some(Info::Time(Duration::from_millis(tok::parse(tokens, warn)?))),
        "nodes" => Some(Info::Nodes(tok::parse(tokens, warn)?)),
        "pv" => Some(Info::Pv(movevec::parse(tokens, true, &mut warn.adapt()))),
        "multipv" => Some(Info::MultiPv(tok::parse(tokens, warn)?)),
        "score" => Some(Info::Score(score::parse(tokens, &mut warn.adapt())?)),
        "currmove" => Some(Info::CurrMove(tok::parse(tokens, warn)?)),
        "currmovenumber" => Some(Info::CurrMoveNumber(tok::parse(tokens, warn)?)),
        "hashfull" => Some(Info::HashFull(make_permille(
            tok::parse(tokens, warn)?,
            warn,
        ))),
        "nps" => Some(Info::Nps(tok::parse(tokens, warn)?)),
        "tbhits" => Some(Info::TbHits(tok::parse(tokens, warn)?)),
        "sbhits" => Some(Info::SbHits(tok::parse(tokens, warn)?)),
        "cpuload" => Some(Info::CpuLoad(make_permille(
            tok::parse(tokens, warn)?,
            warn,
        ))),
        "refutation" => Some(Info::Refutation(movevec::parse(
            tokens,
            true,
            &mut warn.adapt(),
        ))),
        "currline" => {
            let cpu_num = tok::parse(tokens, warn)?;
            let moves = movevec::parse(tokens, true, &mut warn.adapt());
            Some(Info::CurrLine { cpu_num, moves })
        }
        tok => {
            warn.warn(Error::UnexpectedToken(tok.to_string()));
            None
        }
    }
}

pub fn fmt(src: &Info, f: &mut impl PushTokens) {
    match src {
        Info::Depth(val) => f.push_tag("depth", val),
        Info::SelDepth(val) => f.push_tag("seldepth", val),
        Info::Time(val) => f.push_tag("time", &val.as_millis()),
        Info::Nodes(val) => f.push_tag("nodes", val),
        Info::Pv(moves) => {
            f.push_kw("pv");
            movevec::fmt(moves, f);
        }
        Info::MultiPv(val) => f.push_tag("multipv", val),
        Info::Score(val) => {
            f.push_kw("score");
            score::fmt(val, f);
        }
        Info::CurrMove(val) => f.push_tag("currmove", val),
        Info::CurrMoveNumber(val) => f.push_tag("currmovenumber", val),
        Info::HashFull(val) => f.push_tag("hashfull", &val.amount()),
        Info::Nps(val) => f.push_tag("nps", val),
        Info::TbHits(val) => f.push_tag("tbhits", val),
        Info::SbHits(val) => f.push_tag("sbhits", val),
        Info::CpuLoad(val) => f.push_tag("cpuload", &val.amount()),
        Info::Refutation(moves) => {
            f.push_kw("refutation");
            movevec::fmt(moves, f);
        }
        Info::CurrLine { cpu_num, moves } => {
            f.push_tag("currline", cpu_num);
            movevec::fmt(moves, f);
        }
    }
}
