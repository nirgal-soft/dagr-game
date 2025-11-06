use anyhow::Result;
use serde_json::json;
use dagr_lib::ems::entity_manager::EntityManager;
use dagr_lib::components::world::hex::Hex;
use dagr_lib::core::registry::EntityKind;

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
    let seed_hex = self.entity_manager.create_entity(EntityKind::Hex, json!({"x": x, "y": y})).await?;
    let mut prev = self.entity_manager.get_component::<Hex, _>(seed_hex)?;
    for dy in y..100{
      for dx in x..100{
        let entity = self.entity_manager.create_entity(
          EntityKind::Hex,
          json!({
            "x": dx,
            "y": dy,
            "prev": prev
          })).await?;
        prev = self.entity_manager.get_component::<Hex, _>(entity)?;
      }
    }
    Ok(())
  }
}
