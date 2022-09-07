use super::super::{msg::Info, types::Permille};
use super::{movevec, prelude::*, score, tok};

#[derive(Clone, Debug, Error, Eq, PartialEq)]
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

fn make_permille(val: u64, warn: &mut impl Sink<Error>) -> Permille {
    if val >= 1000 {
        warn.warn(Error::PermilleTruncated { src_value: val });
    }
    Permille::new_truncated(val)
}

pub fn parse(tokens: &mut &[&Token], warn: &mut impl Sink<Error>) -> Option<Info> {
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
        Info::Depth(val) => {
            f.do_tok("depth");
            f.do_tok(&val.to_string());
        }
        Info::SelDepth(val) => {
            f.do_tok("seldepth");
            f.do_tok(&val.to_string());
        }
        Info::Time(val) => {
            f.do_tok("time");
            f.do_tok(&val.as_millis().to_string());
        }
        Info::Nodes(val) => {
            f.do_tok("nodes");
            f.do_tok(&val.to_string());
        }
        Info::Pv(moves) => {
            f.do_tok("pv");
            movevec::fmt(moves, f);
        }
        Info::MultiPv(val) => {
            f.do_tok("multipv");
            f.do_tok(&val.to_string());
        }
        Info::Score(val) => {
            f.do_tok("score");
            score::fmt(val, f);
        }
        Info::CurrMove(val) => {
            f.do_tok("currmove");
            f.do_tok(&val.to_string());
        }
        Info::CurrMoveNumber(val) => {
            f.do_tok("currmovenumber");
            f.do_tok(&val.to_string());
        }
        Info::HashFull(val) => {
            f.do_tok("hashfull");
            f.do_tok(&val.amount().to_string());
        }
        Info::Nps(val) => {
            f.do_tok("nps");
            f.do_tok(&val.to_string());
        }
        Info::TbHits(val) => {
            f.do_tok("tbhits");
            f.do_tok(&val.to_string());
        }
        Info::SbHits(val) => {
            f.do_tok("sbhits");
            f.do_tok(&val.to_string());
        }
        Info::CpuLoad(val) => {
            f.do_tok("cpuload");
            f.do_tok(&val.amount().to_string());
        }
        Info::Refutation(moves) => {
            f.do_tok("refutation");
            movevec::fmt(moves, f);
        }
        Info::CurrLine { cpu_num, moves } => {
            f.do_tok("currline");
            f.do_tok(&cpu_num.to_string());
            movevec::fmt(moves, f);
        }
    }
}
