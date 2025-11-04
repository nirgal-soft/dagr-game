use std::collections::HashMap;
use hecs::Entity;

#[derive(Clone, Debug)]
pub struct Map{
  grid: HashMap<(i32, i32), Entity>,
}

impl Map{
  pub fn new() -> Self{
    Self{grid: HashMap::new()}
  }

  pub fn insert(&mut self, pos: (i32, i32), entity: Entity){
    self.grid.insert(pos, entity);
  }

  pub fn get(&self, pos:(i32, i32)) -> Option<Entity>{
    self.grid.get(&pos).copied()
  }

  pub fn clear(&mut self){
    self.grid.clear();
  }
}
