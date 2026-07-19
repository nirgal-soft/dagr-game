use std::{collections::HashSet, sync::Arc};

use anyhow::{anyhow, Result};
use dagr_lib::{
  combat::CombatStrike,
  components::{
    characters::{character::Character, position::CharacterPosition},
    monsters::monster_stats::MonsterStats,
  },
  ems::{component::Component, entity_manager::EntityManager},
  ids::LocationId,
};
use hecs::Entity;
use sqlx::PgPool;

use crate::{areas::{Area, Pos}, pathfinding::a_star::find_path};

use super::controller::{self, EnemyAtTile};

const NOTICE_RADIUS: i32 = 8;

pub struct EnemyTurnReport{
  pub attacks: Vec<(String,CombatStrike)>,
  pub moved: bool,
}

pub async fn run_turns(
  pool:Arc<PgPool>,
  manager:&EntityManager,
  area:&Area,
  location_id:LocationId,
  player:Entity,
  player_pos:Pos,
)->Result<EnemyTurnReport>{
  let enemies=snapshots(manager,location_id)?;
  let mut occupied=enemies.iter().map(|enemy|enemy.pos).collect::<HashSet<_>>();
  let mut report=EnemyTurnReport{attacks:Vec::new(),moved:false};

  for enemy in enemies{
    if controller::hit_points(manager,player).is_some_and(|(hp,_)|hp==0){break}
    let distance=chebyshev(enemy.pos,player_pos);
    if distance<=1{
      let strike=controller::enemy_attack(pool.clone(),manager,&enemy.actor,player).await?;
      report.attacks.push((enemy.actor.name.clone(),strike));
      continue
    }
    if distance>NOTICE_RADIUS || !area.is_visible(enemy.pos.0,enemy.pos.1){continue}
    occupied.remove(&enemy.pos);
    if let Some(destination)=pursuit_step(area,enemy.pos,player_pos,&occupied){
      if destination!=player_pos{
        move_enemy(manager,enemy.actor.entity,destination)?;
        occupied.insert(destination);
        report.moved=true;
      }
    }
  }
  if report.moved{manager.save_dirty(&pool).await?;}
  Ok(report)
}

struct EnemySnapshot{
  actor:EnemyAtTile,
  pos:Pos,
}

fn snapshots(manager:&EntityManager,location_id:LocationId)->Result<Vec<EnemySnapshot>>{
  let world=manager.world();
  let world=world.lock().map_err(|_|anyhow!("ECS world lock is poisoned"))?;
  let mut enemies=world.query::<(&Character,&MonsterStats,&CharacterPosition)>().iter()
    .filter_map(|(entity,(character,_,position))|{
      let position=position.get();
      if position.get_location_id().ok()!=Some(location_id){return None}
      Some(EnemySnapshot{
        actor:EnemyAtTile{
          entity,
          character_id:character.get().get_id().ok()?,
          name:character.get().get_name().to_string(),
        },
        pos:(position.x,position.y),
      })
    }).collect::<Vec<_>>();
  enemies.sort_by_key(|enemy|enemy.actor.character_id.get());
  Ok(enemies)
}

fn move_enemy(manager:&EntityManager,entity:Entity,destination:Pos)->Result<()>{
  let world=manager.world();
  let world=world.lock().map_err(|_|anyhow!("ECS world lock is poisoned"))?;
  let mut position=world.get::<&mut CharacterPosition>(entity)
    .map_err(|_|anyhow!("active enemy has no position"))?;
  let mut data=position.get();
  data.x=destination.0;
  data.y=destination.1;
  position.set(data);
  Ok(())
}

fn pursuit_step(area:&Area,from:Pos,to:Pos,occupied:&HashSet<Pos>)->Option<Pos>{
  find_path(from,to,|x,y|{
    let position=(x,y);
    area.is_walkable(x,y) && (position==to || !occupied.contains(&position))
  }).and_then(|path|path.first().copied())
}

fn chebyshev(a:Pos,b:Pos)->i32{(a.0-b.0).abs().max((a.1-b.1).abs())}

#[cfg(test)]
mod tests{
  use super::*;
  #[test]
  fn diagonal_adjacency_is_melee_range(){
    assert_eq!(chebyshev((2,2),(3,3)),1);
  }

  #[test]
  fn pursuit_takes_one_walkable_unoccupied_step(){
    let area=Area::new(12,12,crate::areas::Ground::DUNGEON);
    let occupied=HashSet::from([(3,2)]);
    let step=pursuit_step(&area,(2,2),(6,2),&occupied).unwrap();
    assert!(!occupied.contains(&step));
    assert_eq!(chebyshev((2,2),step),1);
    assert!(chebyshev(step,(6,2))<chebyshev((2,2),(6,2)));
  }
}
