use std::sync::Arc;

use anyhow::{anyhow, Result};
use dagr_lib::{
  components::{
    characters::{
      character::Character,
      character_types::CharacterType,
      position::CharacterPosition,
    },
    stats::{base_stats::BaseStatsData, pc_stats::PlayerCharacterStatsData},
  },
  ems::{component::Component, entity_manager::EntityManager},
  factories::characters::character::{
    CharacterPositionSeed, PlayerCharacterSeed,
  },
  ids::LocationId,
};
use hecs::Entity;
use sqlx::PgPool;

pub async fn ensure(
  manager: &EntityManager,
  location_id: LocationId,
  spawn: (i32, i32),
) -> Result<Entity>{
  if let Some(entity) = find(manager, location_id){
    return Ok(entity)
  }
  let mut stats = BaseStatsData::default();
  stats.set_av(10);
  stats.set_dv(0);
  manager.create(PlayerCharacterSeed{
    name: "Arena fighter".to_string(),
    base_stats: stats,
    pc_stats: PlayerCharacterStatsData::default(),
    position: Some(CharacterPositionSeed{
      location_id,
      x: spawn.0,
      y: spawn.1,
    }),
  }).await
}

pub fn find(manager: &EntityManager, location_id: LocationId) -> Option<Entity>{
  let world = manager.world();
  let world = world.lock().ok()?;
  world.query::<(&Character, &CharacterPosition)>().iter().find_map(
    |(entity, (character, position))|{
      let is_player = character.get().get_ctype() == &CharacterType::Player;
      let is_here = position.get().get_location_id().ok() == Some(location_id);
      (is_player && is_here).then_some(entity)
    },
  )
}

pub fn position(manager: &EntityManager, entity: Entity) -> Result<(i32, i32)>{
  let world = manager.world();
  let world = world.lock().map_err(|_| anyhow!("ECS world lock is poisoned"))?;
  let position = world.get::<&CharacterPosition>(entity)
    .map_err(|_| anyhow!("arena player has no position"))?
    .get();
  Ok((position.x, position.y))
}

pub async fn move_to(
  manager: &EntityManager,
  pool: &Arc<PgPool>,
  entity: Entity,
  destination: (i32, i32),
) -> Result<()>{
  {
    let world = manager.world();
    let world = world.lock().map_err(|_| anyhow!("ECS world lock is poisoned"))?;
    let mut position = world.get::<&mut CharacterPosition>(entity)
      .map_err(|_| anyhow!("arena player has no position"))?;
    let mut data = position.get();
    data.x = destination.0;
    data.y = destination.1;
    position.set(data);
  }
  manager.save_dirty(pool).await?;
  Ok(())
}
