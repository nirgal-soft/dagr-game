use dagr_lib::components::world::location::LocationType;
use dagr_lib::ids::LocationId;
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

#[derive(Debug, Error)]
pub enum ViewError {
  #[error("no current location")]
  NoCurrentLocation,
  #[error("no parent location found")]
  NoParentLocation,
  #[error("location not found: {0}")]
  LocationNotFound(LocationId),
  #[error("area not in cache")]
  AreaNotCached,
  #[error("unsupported location type: {0:?}")]
  UnsupportedLocationType(LocationType),
  #[error("generation failed: {0}")]
  GenerationFailed(String),
}
