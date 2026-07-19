use std::{collections::HashSet, sync::Arc};

use anyhow::{anyhow, Result};
use dagr_lib::{
  combat::{CombatService, CombatStrike},
  components::{
    characters::{character::Character, position::CharacterPosition},
    monsters::monster_stats::MonsterStats,
    stats::base_stats::BaseStats,
  },
  ems::{component::Component, entity_manager::EntityManager},
  ids::{CharacterId, LocationId},
};
use hecs::Entity;
use sqlx::PgPool;

#[derive(Clone, Debug)]
pub struct EnemyAtTile{
  pub entity: Entity,
  pub character_id: CharacterId,
  pub name: String,
}

pub fn enemy_at(
  manager: &EntityManager,
  location_id: LocationId,
  x: i32,
  y: i32,
) -> Option<EnemyAtTile>{
  let world = manager.world();
  let world = world.lock().ok()?;
  world.query::<(&Character, &MonsterStats, &CharacterPosition)>().iter().find_map(
    |(entity, (character, _, position))|{
      let position = position.get();
      let matches = position.get_location_id().ok() == Some(location_id)
        && position.x == x
        && position.y == y;
      if !matches{return None}
      Some(EnemyAtTile{
        entity,
        character_id: character.get().get_id().ok()?,
        name: character.get().get_name().to_string(),
      })
    },
  )
}

pub fn enemy_positions(
  manager:&EntityManager,
  location_id:LocationId,
)->HashSet<(i32,i32)>{
  let world=manager.world();
  let Ok(world)=world.lock() else{return HashSet::new()};
  world.query::<(&MonsterStats,&CharacterPosition)>().iter()
    .filter_map(|(_,(_,position))|{
      let position=position.get();
      (position.get_location_id().ok()==Some(location_id)).then_some((position.x,position.y))
    }).collect()
}

pub fn character_id(manager: &EntityManager, entity: Entity) -> Result<CharacterId>{
  Ok(manager.get_component::<Character, _>(entity)?.get().get_id()?)
}

pub fn hit_points(manager: &EntityManager, entity: Entity) -> Option<(i32, i32)>{
  manager.get_component::<BaseStats, _>(entity).ok().map(|stats|{
    let stats=stats.get();
    (stats.current_hp,stats.max_hp)
  })
}

pub async fn reset_arena(
  pool:Arc<PgPool>,
  manager:&EntityManager,
  player:Entity,
  location_id:LocationId,
)->Result<()>{
  let player_id=character_id(manager,player)?;
  CombatService::new(pool).reset_arena(player_id,location_id).await?;
  let world=manager.world();
  let mut world=world.lock().map_err(|_|anyhow!("ECS world lock is poisoned"))?;
  let enemies=world.query::<(&MonsterStats,&CharacterPosition)>().iter()
    .filter_map(|(entity,(_,position))|
      (position.get().get_location_id().ok()==Some(location_id)).then_some(entity))
    .collect::<Vec<_>>();
  for entity in enemies{world.remove_one::<CharacterPosition>(entity)?;}
  let max_hp=world.get::<&BaseStats>(player)
    .map_err(|_|anyhow!("arena player has no base stats"))?.get().max_hp;
  replace_hp(&mut world,player,max_hp)?;
  Ok(())
}

pub async fn player_attack(
  pool:Arc<PgPool>,
  manager:&EntityManager,
  player:Entity,
  enemy:EnemyAtTile,
)->Result<(CombatStrike,String)>{
  let player_id=character_id(manager,player)?;
  let strike=CombatService::new(pool).strike(player_id,enemy.character_id).await?;
  {
    let world=manager.world();
    let mut world=world.lock().map_err(|_|anyhow!("ECS world lock is poisoned"))?;
    replace_hp(&mut world,enemy.entity,strike.defender_hp)?;
    if strike.defender_defeated{
      world.remove_one::<CharacterPosition>(enemy.entity)
        .map_err(|_|anyhow!("defeated enemy position disappeared"))?;
    }
  }
  Ok((strike,enemy.name))
}

pub async fn enemy_attack(
  pool:Arc<PgPool>,
  manager:&EntityManager,
  enemy:&EnemyAtTile,
  player:Entity,
)->Result<CombatStrike>{
  let player_id=character_id(manager,player)?;
  let strike=CombatService::new(pool).strike(enemy.character_id,player_id).await?;
  let world=manager.world();
  let mut world=world.lock().map_err(|_|anyhow!("ECS world lock is poisoned"))?;
  replace_hp(&mut world,player,strike.defender_hp)?;
  Ok(strike)
}

fn replace_hp(world:&mut hecs::World,entity:Entity,current_hp:i32)->Result<()>{
  let mut stats=world.get::<&mut BaseStats>(entity)
    .map_err(|_| anyhow!("combatant has no base stats"))?;
  let mut data=stats.get();
  data.current_hp=current_hp;
  stats.set(data);
  stats.clean();
  Ok(())
}
