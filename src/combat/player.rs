use std::sync::Arc;

use anyhow::{anyhow, Result};
use dagr_lib::{
  components::{
    characters::{
      character::Character,
      character_types::CharacterType,
      position::CharacterPosition,
    },
    stats::{base_stats::{BaseStats, BaseStatsData}, pc_stats::PlayerCharacterStatsData},
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
  pool: &Arc<PgPool>,
  location_id: LocationId,
  spawn: (i32, i32),
) -> Result<Entity>{
  if let Some(entity) = find(manager, location_id){
    upgrade_fixture(manager,pool,entity).await?;
    return Ok(entity)
  }
  manager.create(PlayerCharacterSeed{
    name: "Arena fighter".to_string(),
    base_stats: arena_stats(),
    pc_stats: PlayerCharacterStatsData::default(),
    position: Some(CharacterPositionSeed{
      location_id,
      x: spawn.0,
      y: spawn.1,
    }),
  }).await
}

fn arena_stats()->BaseStatsData{
  let mut stats=BaseStatsData::default();
  stats.set_hd((5,6,0));
  stats.set_av(13);
  stats.set_dv(5);
  stats.set_sv(9);
  stats.set_current_hp(18);
  stats.set_max_hp(18);
  stats.set_level(5);
  stats
}

async fn upgrade_fixture(manager:&EntityManager,pool:&Arc<PgPool>,entity:Entity)->Result<()>{
  let current=manager.get_component::<dagr_lib::components::stats::base_stats::BaseStats,_>(entity)?;
  let current=current.get();
  if current.get_level()==5{return Ok(())}
  let mut upgraded=arena_stats();
  upgraded.set_character_id(dagr_lib::ids::CharacterId::new(current.get_character_id())?);
  manager.set_component::<BaseStats,_>(entity,upgraded)?;
  manager.save_dirty(pool).await?;
  Ok(())
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

#[cfg(test)]
mod tests{
  use super::*;

  #[test]
  fn arena_fixture_uses_level_five_strong_baseline(){
    let stats=arena_stats();
    assert_eq!(stats.get_level(),5);
    assert_eq!(stats.get_av(),13);
    assert_eq!(stats.get_dv(),5);
    assert_eq!(stats.get_sv(),9);
    assert_eq!((stats.get_current_hp(),stats.get_max_hp()),(18,18));
  }
}
