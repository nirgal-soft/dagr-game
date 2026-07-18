use anyhow::Result;
use dagr_lib::components::world::hex::Hex;
use dagr_lib::ems::{component::Component, entity_manager::EntityManager};
use dagr_lib::factories::world::hex::HexSeed;

pub struct RegionGenerator{
  pub entity_manager: EntityManager,
}

impl RegionGenerator{
  pub fn new(em: EntityManager) -> Self{
    Self{entity_manager: em}
  }

  pub async fn generate(&mut self) -> Result<()>{
    let x = 1;
    let y = 1;
    let seed_hex = self.entity_manager.create(HexSeed{x, y, prev: None}).await?;
    let mut prev = self.entity_manager.get_component::<Hex, _>(seed_hex)?;
    for dy in y..100{
      for dx in x..100{
        let entity = self.entity_manager.create(HexSeed{
          x: dx,
          y: dy,
          prev: Some(prev.clone()),
        }).await?;
        prev = self.entity_manager.get_component::<Hex, _>(entity)?;
      }
    }
    Ok(())
  }
}
