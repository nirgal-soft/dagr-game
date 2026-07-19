use dagr_lib::{
  factories::world::hex::HexProfileSeed,
  kits::hexkit::{
    climate::Climate,
    poi::PointOfInterest,
    terrain::{Terrain,Vegetation,Water},
  },
};

use crate::areas::Pos;

const FIELD_COUNT:usize=8;

pub struct HexEditor{
  pub cursor:Pos,
  selected:usize,
  terrain:usize,
  vegetation:usize,
  water:usize,
  poi:usize,
  climate:usize,
  danger:i32,
  resources:i32,
}

impl HexEditor{
  pub fn new(cursor:Pos)->Self{
    Self{cursor,selected:0,terrain:2,vegetation:2,water:2,poi:0,climate:0,danger:1,resources:1}
  }
  pub fn selected(&self)->usize{self.selected}
  pub fn move_cursor(&mut self,dx:i32,dy:i32){self.cursor.0+=dx;self.cursor.1+=dy;}
  pub fn select_field(&mut self,delta:i32){
    self.selected=(self.selected as i32+delta).rem_euclid(FIELD_COUNT as i32) as usize;
  }
  pub fn cycle(&mut self,delta:i32){
    match self.selected{
      0=>self.terrain=cycle(self.terrain,4,delta),
      1=>self.vegetation=cycle(self.vegetation,4,delta),
      2=>self.water=cycle(self.water,3,delta),
      3=>self.poi=cycle(self.poi,8,delta),
      4=>self.climate=cycle(self.climate,3,delta),
      5=>self.danger=(self.danger+delta).clamp(0,10),
      6=>self.resources=(self.resources+delta).clamp(0,10),
      _=>{},
    }
  }
  pub fn profile(&self)->HexProfileSeed{
    HexProfileSeed{
      terrain:[Terrain::Mountains,Terrain::Hills,Terrain::Plains,Terrain::Swamp][self.terrain],
      vegetation:[Vegetation::DenseForest,Vegetation::LightForest,Vegetation::Grassland,Vegetation::Barren][self.vegetation],
      water:[Water::Lake,Water::River,Water::Dry][self.water],
      poi:[PointOfInterest::Ruins,PointOfInterest::Caves,PointOfInterest::Tomb,PointOfInterest::Lair,PointOfInterest::NatResource(dagr_lib::kits::hexkit::poi::NaturalResource::IronBog),PointOfInterest::Settlement,PointOfInterest::Mine,PointOfInterest::Dungeon][self.poi].clone(),
      climate:[Climate::Temperate,Climate::Tropical,Climate::Tundra][self.climate],
      region_id:None,
      danger_level:self.danger,
      resource_richness:self.resources,
    }
  }
  pub fn lines(&self,visible:usize)->Vec<String>{
    let profile=self.profile();
    let values=[
      format!("Terrain: {}",profile.terrain),
      format!("Vegetation: {}",profile.vegetation),
      format!("Water: {}",profile.water),
      format!("POI: {}",poi_name(&profile.poi)),
      format!("Climate: {}",climate_name(profile.climate)),
      format!("Danger: {}",profile.danger_level),
      format!("Resources: {}",profile.resource_richness),
      "Region: None".to_string(),
    ];
    let start=self.selected.saturating_sub(visible.saturating_sub(1))
      .min(FIELD_COUNT.saturating_sub(visible));
    values.into_iter().enumerate().skip(start).take(visible).map(|(index,value)|
      format!("{} {}",if index==self.selected{">"}else{" "},value)).collect()
  }
}

fn cycle(current:usize,len:usize,delta:i32)->usize{(current as i32+delta).rem_euclid(len as i32) as usize}
fn climate_name(value:Climate)->&'static str{match value{Climate::Temperate=>"Temperate",Climate::Tropical=>"Tropical",Climate::Tundra=>"Tundra"}}
fn poi_name(value:&PointOfInterest)->&'static str{match value{PointOfInterest::Ruins=>"Ruins",PointOfInterest::Caves=>"Caves",PointOfInterest::Tomb=>"Tomb",PointOfInterest::Lair=>"Lair",PointOfInterest::NatResource(_)=>"Natural resource",PointOfInterest::Settlement=>"Settlement",PointOfInterest::Mine=>"Mine",PointOfInterest::Dungeon=>"Dungeon"}}

#[cfg(test)]
mod tests{
  use super::*;

  #[test]
  fn editor_cycles_profile_without_moving_cursor(){
    let mut editor=HexEditor::new((4,5));
    editor.cycle(1);
    assert_eq!(editor.profile().terrain,Terrain::Swamp);
    assert_eq!(editor.cursor,(4,5));
    editor.select_field(7);
    assert!(editor.lines(5).iter().any(|line|line.contains("Region")));
  }
}
