use std::num::NonZeroU64;
use std::time::Duration;

use owlchess::moves::UciMove;
use owlchess::RawBoard;

use crate::score::BoundedRelScore;

use super::str::{OptComboVar, OptName, RegisterName, UciString};
use super::types::{Permille, TriStatus};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Register {
    Later,
    Now { name: RegisterName, code: UciString },
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Go {
    pub searchmoves: Option<Vec<UciMove>>,
    pub ponder: Option<()>,
    pub infinite: Option<()>,
    pub wtime: Option<Duration>,
    pub btime: Option<Duration>,
    pub winc: Option<Duration>,
    pub binc: Option<Duration>,
    pub movestogo: Option<NonZeroU64>,
    pub mate: Option<u64>,
    pub depth: Option<u64>,
    pub nodes: Option<u32>,
    pub movetime: Option<Duration>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Command {
    Uci,
    Debug(bool),
    IsReady,
    SetOption {
        name: OptName,
        value: Option<UciString>,
    },
    Register(Register),
    UciNewGame,
    Position {
        startpos: RawBoard,
        moves: Vec<UciMove>,
    },
    Go(Go),
    Stop,
    PonderHit,
    Quit,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Id {
    Name(UciString),
    Author(UciString),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Info {
    Depth(u32),
    SelDepth(u32),
    Time(Duration),
    Nodes(u64),
    Pv(Vec<UciMove>),
    MultiPv(u32),
    Score(BoundedRelScore),
    CurrMove(UciMove),
    CurrMoveNumber(u32),
    HashFull(Permille),
    Nps(u64),
    TbHits(u64),
    SbHits(u64),
    CpuLoad(Permille),
    Refutation(Vec<UciMove>),
    CurrLine { cpu_num: u32, moves: Vec<UciMove> },
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum OptBody {
    Check(bool),
    Spin {
        default: i64,
        min: i64,
        max: i64,
    },
    Combo {
        default: OptComboVar,
        vars: Vec<OptComboVar>,
    },
    Button,
    String(UciString),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
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
    Info {
        info: Vec<Info>,
        string: Option<UciString>,
    },
    Option {
        name: OptName,
        body: OptBody,
    },
}
