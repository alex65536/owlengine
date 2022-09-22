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
    Failure,
}

// TODO
