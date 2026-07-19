use std::sync::Arc;

use anyhow::Result;
use dagr_lib::{
  ems::entity_manager::EntityManager,
  ids::LocationId,
  ops::monsters::definition::MonsterDefinitionOps,
};
use hecs::Entity;
use sqlx::PgPool;

use super::{controller, picker::{MonsterChoice, MonsterPicker}, player};

pub struct CombatSession{
  pool: Arc<PgPool>,
  player: Option<Entity>,
  picker: Option<MonsterPicker>,
}

impl CombatSession{
  pub fn new(pool: Arc<PgPool>) -> Self{
    Self{pool,player:None,picker:None}
  }

  pub fn pool(&self) -> Arc<PgPool>{self.pool.clone()}
  pub fn player(&self) -> Option<Entity>{self.player}
  pub fn picker(&self) -> Option<&MonsterPicker>{self.picker.as_ref()}
  pub fn picker_is_open(&self) -> bool{self.picker.is_some()}

  pub async fn enter_arena(
    &mut self,
    manager:&EntityManager,
    location_id:LocationId,
    spawn:(i32,i32),
  )->Result<(i32,i32)>{
    let entity=player::ensure(manager,location_id,spawn).await?;
    self.player=Some(entity);
    player::position(manager,entity)
  }

  pub fn hit_points(&self,manager:&EntityManager)->Option<(i32,i32)>{
    self.player.and_then(|entity|controller::hit_points(manager,entity))
  }

  pub async fn move_player(
    &self,
    manager:&EntityManager,
    destination:(i32,i32),
  )->Result<()>{
    if let Some(entity)=self.player{
      player::move_to(manager,&self.pool,entity,destination).await?;
    }
    Ok(())
  }

  pub async fn reset_arena(
    &self,
    manager:&EntityManager,
    location_id:LocationId,
  )->Result<()>{
    if let Some(player)=self.player{
      controller::reset_arena(self.pool.clone(),manager,player,location_id).await?;
    }
    Ok(())
  }

  pub async fn open_picker(&mut self)->Result<()>{
    let definitions=MonsterDefinitionOps::new(self.pool.clone()).list().await?;
    self.picker=Some(MonsterPicker::new(definitions));
    Ok(())
  }

  pub fn close_picker(&mut self){self.picker=None}
  pub fn picker_input(&mut self,character:char){
    if let Some(picker)=self.picker.as_mut(){picker.input(character)}
  }
  pub fn picker_backspace(&mut self){
    if let Some(picker)=self.picker.as_mut(){picker.backspace()}
  }
  pub fn picker_move(&mut self,delta:i32){
    if let Some(picker)=self.picker.as_mut(){picker.move_selection(delta)}
  }
  pub fn selected_monster(&self)->Option<MonsterChoice>{
    self.picker.as_ref()?.selected().cloned()
  }
}
