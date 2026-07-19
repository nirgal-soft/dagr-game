use crate::areas::{Area, Pos};

use super::model::{InspectionEntry, InspectionReport};

pub fn inspect(area:&Area,pos:Pos,actor:Option<&str>)->InspectionReport{
  let mut entries=Vec::new();
  if let Some(actor)=actor{
    entries.push(InspectionEntry::new(actor,"A creature occupying this tile."));
  }
  for fixture in area.fixtures_at(pos.0,pos.1).iter().rev(){
    entries.push(InspectionEntry::new(&fixture.name,&fixture.description));
  }
  if let Some(poi)=area.get_poi_at(pos){
    entries.push(InspectionEntry::new(&poi.label,poi.kind.label()));
  }
  if let Some(feature)=area.get_feature(pos.0,pos.1){
    let (name,description)=feature.inspection();
    entries.push(InspectionEntry::new(name,description));
  }
  let (name,description)=area.ground().inspection();
  entries.push(InspectionEntry::new(name,description));
  InspectionReport{pos,entries}
}

#[cfg(test)]
mod tests{
  use super::*;
  use crate::areas::{Fixture,FixtureKind};

  #[test]
  fn fixture_description_is_layered_above_ground(){
    let mut area=Area::wilderness(10,10);
    area.add_fixture(Fixture::new(
      (4,4),FixtureKind::Bedroll,"Tattered bedroll","A damp woolen bedroll.",
    ));
    let report=inspect(&area,(4,4),None);
    assert_eq!(area.get_tile(4,4).symbol,'=');
    assert_eq!(report.entries[0].name,"Tattered bedroll");
    assert!(report.entries.iter().any(|entry|entry.name=="Forest floor"));
  }
}
