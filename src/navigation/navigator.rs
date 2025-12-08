use std::collections::VecDeque;
use anyhow::{anyhow, Result};
use tracing::info;
use crate::areas::{Area, Pos};
use crate::errors::NavigationError;
use crate::pathfinding::a_star::find_path_to_nearest;

pub struct Navigator{
  path_queue: VecDeque<Pos>,
  is_auto_exploring: bool,
}

impl Navigator{
  pub fn new() -> Self{
    Self{
      path_queue: VecDeque::new(),
      is_auto_exploring: false,
    }
  }

  pub fn is_navigating(&self) -> bool{
    !self.path_queue.is_empty()
  }

  pub fn is_exploring(&self) -> bool{
    self.is_auto_exploring
  }

  pub fn cancel(&mut self){
    self.path_queue.clear();
    self.is_auto_exploring = false;
  }

  pub fn set_path(&mut self, path: Vec<Pos>){
    self.path_queue = path.into_iter().collect();
  }

  pub fn next_step(&mut self) -> Option<Pos>{
    self.path_queue.pop_front()
  }

  pub fn start_exploring(&mut self){
    self.is_auto_exploring = true;
  }

  pub fn stop_exploring(&mut self){
    self.is_auto_exploring = false;
  }

  pub fn queue_is_empty(&self) -> bool{
    self.path_queue.is_empty()
  }

  pub fn navigate_to_nearest(
    &mut self,
    start: Pos,
    goals: Vec<Pos>,
    area: &Area,
  ) -> Result<()>{
    if goals.is_empty(){
      return Err(anyhow!(NavigationError::NoGoalsVisible));
    }

    let result = find_path_to_nearest(start, goals, |x, y|{
      area.is_seen_and_walkable(x, y)
    });

    match result{
      Some((path, _)) => {
        self.set_path(path);
        Ok(())
      }
      None => Err(anyhow!(NavigationError::NoPathFound)),
    }
  }

  pub fn find_exploration_target(
    &mut self,
    start: Pos,
    area: &Area,
  ) -> Result<()>{
    if !self.is_auto_exploring{
      return Err(anyhow!(NavigationError::NotExploring));
    }

    let frontiers = area.find_exploration_frontiers();
    
    info!("auto-explore: found {} frontiers", frontiers.len());

    if frontiers.is_empty(){
      self.is_auto_exploring = false;
      return Err(anyhow!(NavigationError::FullyExplored));
    }

    info!("total tiles seen: {}", area.seen_count());

    let result = find_path_to_nearest(start, frontiers.clone(), |x, y|{
      area.is_seen_and_walkable(x, y)
    });

    match result{
      Some((path, goal)) =>{
        info!("found path to {:?} with {} steps", goal, path.len());
        self.set_path(path);
        Ok(())
      }
      None => {
        info!(
          "pathfinding failed. first few frontiers: {:?}",
          &frontiers[..frontiers.len().min(5)]
        );
        self.is_auto_exploring = false;
        Err(anyhow!(NavigationError::NoPathFound))
      }
    }
  }
}

impl Default for Navigator{
  fn default() -> Self{
    Self::new()
  }
}
