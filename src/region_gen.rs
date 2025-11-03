use anyhow::Result;
use serde_json::json;
use dagr_lib::ems::entity_manager::EntityManager;
use dagr_lib::db::connection;
use dagr_lib::core::registry::{EntityKind, FactoryRegistry};
use dagr_lib::bootstrap::{build_factor_registry, AppConfig};
use hecs::{World, Entity};

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
    Ok(())
  }
}
