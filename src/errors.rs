use thiserror::Error;

#[derive(Debug, Error)]
pub enum NavigationError{
  #[error("no target visible")]
  NoGoalsVisible,
  #[error("no path found")]
  NoPathFound,
  #[error("not currently exploring")]
  NotExploring,
  #[error("area fully explored")]
  FullyExplored,
}

#[derive(Debug, Error)]
pub enum ViewError{
  #[error("not in wildenress")]
  NotInWilderness,
  #[error("not in dungeon")]
  NotInDungeon,
  #[error("no parent locaiton found")]
  NoParentLocation,
  #[error("no hex found at location")]
  HexNotFound,
  #[error("no dungeon found at location")]
  DungeonNotFound,
  #[error("no wilderness found at location")]
  WildernessNotFound,
  #[error("area not in cache")]
  AreaNotInCache,
  #[error("failed to generate area: {0}")]
  GenerationFailed(String),
}

#[derive(Debug, Error)]
pub enum GameError{
  #[error("no hex at player position")]
  NoHexAtPosition,
  #[error("navigation error: {0}")]
  Navigation(#[from] NavigationError),
  #[error("view error: {0}")]
  View(#[from] ViewError),
}
