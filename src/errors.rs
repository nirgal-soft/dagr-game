use thiserror::Error;

#[derive(Debug, Error)]
pub enum NavigationError {
    #[error("no target visible")]
    NoGoalsVisible,
    #[error("no path found")]
    NoPathFound,
    #[error("not currently exploring")]
    NotExploring,
    #[error("area fully explored")]
    FullyExplored,
}
