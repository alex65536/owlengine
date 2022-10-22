use async_trait::async_trait;

use crate::score::BoundedRelScore;

use owlchess::{Color, Move, MoveChain};

use std::time::Duration;
use std::sync::Arc;

use tokio::sync::{mpsc, watch};

use thiserror::Error;

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub enum AnalysisState {
    Running,
    Stopping,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    Starting,
    Thinking,
    Pondering,
    PonderStopping,
    Waiting,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub enum ExitState {
    Exiting,
    Exited,
    Killed,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub enum State {
    Idle,
    Analysis(AnalysisState),
    Game(GameState),
    Exit(ExitState),
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    // TODO
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub struct Limits {
    // TODO
}


#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub struct Clock {
    pub white_time: Duration,
    pub black_time: Duration,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub struct TimeControl {
    // TODO
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum OptType {
    Bool,
    Int {
        min: i64,
        max: i64,
    },
    Enum(Vec<String>),
    String,
    Action,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum OptValue {
    Bool(bool),
    Int(i64),
    Enum(String),
    String(String),
    Action,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub struct OptDesc {
    pub name: String,
    pub ty: OptType,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub struct AnalysisStatus {
    pub depth: Option<u32>,
    pub time: Option<Duration>,
    pub nodes: Option<u64>,
    pub nps: Option<u64>,
    pub pv: Vec<Move>,
    pub score: Option<BoundedRelScore>,
}

pub struct Analysis {
    game: Arc<dyn Engine>,
    status: mpsc::Receiver<AnalysisStatus>,
    bestmove: watch::Receiver<Move>,
}

pub struct AnalysisSender {
    status: mpsc::Sender<AnalysisStatus>,
    bestmove: watch::Sender<Move>,
}

pub fn analysis<E: Engine>(game: Arc<E>) -> (Analysis, AnalysisSender) {
    // TODO
    todo!()
}

impl Analysis {
    #[inline]
    pub async fn wait(&self) -> Move {
        // TODO
        todo!()
    }
}

pub struct Game {
    // TODO
}

// TODO : consider how to add "debug on" and "debug off" -> with separate method or with option?
// TODO : pipes with types warnings

#[async_trait]
pub trait Engine {
    async fn exit(&self) -> Result<(), Error>;
    fn kill(&self);
    fn state(&self) -> Result<State, Error>;
    fn start_analysis(&self, limits: Limits, pos: MoveChain) -> Analysis;
    fn stop_analysis(&self);
    fn set_ponder(&self, ponder: bool);
    async fn ping(&self) -> Result<(), Error>;
    async fn start_game(&self, pos: MoveChain, our_color: Color, clock: Clock, control: TimeControl) -> Game;
    fn stop_game(&self);
    async fn report_move(&self, mv: Move, clock: Option<Clock>) -> Result<(), Error>;
    fn options(&self) -> &[OptDesc];
    fn get_option(&self, name: &str) -> OptValue;
    async fn set_option(&self, name: &str, value: OptValue) -> Result<(), Error>;
}
