use std::num::NonZeroU64;
use std::time::Duration;

use owlchess::moves::UciMove;
use owlchess::RawBoard;

use crate::score::BoundedRelScore;

use super::str::{OptEnumValue, OptName, RegisterName, UciStr};
use super::types::{Permille, TriStatus};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Register {
    Later,
    Now { name: RegisterName, code: UciStr },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum GoLimits {
    Infinite,
    Clock {
        wtime: Duration,
        btime: Duration,
        winc: Duration,
        binc: Duration,
        movestogo: Option<NonZeroU64>,
    },
    Mate(u64),
    Limits {
        depth: Option<u64>,
        nodes: Option<u64>,
        movetime: Option<Duration>,
    },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Command {
    Uci,
    Debug(bool),
    IsReady,
    SetOption {
        name: OptName,
        value: UciStr,
    },
    Register(Register),
    UciNewGame,
    Position {
        startpos: RawBoard,
        moves: Vec<UciMove>,
    },
    Go {
        searchmoves: Option<Vec<UciMove>>,
        ponder: Option<()>,
        limits: GoLimits,
    },
    Stop,
    PonderHit,
    Quit,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Id {
    Name(UciStr),
    Author(UciStr),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Info {
    Depth(u32),
    SelDepth(u32),
    Time(Duration),
    Nodes(u64),
    Pv(Vec<UciMove>),
    MultiPv(u32),
    Score(BoundedRelScore),
    CurMove(UciMove),
    CurMoveNumber(u32),
    HashFull(Permille),
    Nps(u64),
    TbHits(u64),
    SbHits(u64),
    CpuLoad(Permille),
    String(UciStr),
    Refutation(Vec<UciMove>),
    CurrLine(Vec<UciMove>),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum OptBody {
    Bool(bool),
    Int {
        default: i64,
        min: i64,
        max: i64,
        var: i64,
    },
    Enum {
        default: OptEnumValue,
        vals: Vec<OptEnumValue>,
    },
    Action,
    String(UciStr),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Message {
    Id(Id),
    UciOk,
    ReadyOk,
    BestMove {
        bestmove: UciMove,
        ponder: Option<UciMove>,
    },
    CopyProtection(TriStatus),
    Registration(TriStatus),
    Info(Vec<Info>),
    Option {
        name: OptName,
        body: OptBody,
    },
}
