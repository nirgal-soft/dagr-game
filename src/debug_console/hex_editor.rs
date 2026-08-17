use dagr_lib::{content::ContentKey, world::HexProfile};

use crate::areas::Pos;

const FIELD_COUNT: usize = 8;
const TERRAIN: [&str; 4] = ["mountains", "hills", "plains", "swamp"];
const VEGETATION: [&str; 4] = ["dense_forest", "light_forest", "grassland", "barren"];
const WATER: [&str; 3] = ["lake", "river", "dry"];
const POINTS_OF_INTEREST: [&str; 8] = [
  "ruins",
  "caves",
  "tomb",
  "lair",
  "natural_resource",
  "settlement",
  "mine",
  "dungeon",
];
const CLIMATE: [&str; 3] = ["temperate", "tropical", "tundra"];

pub struct HexEditor {
  pub cursor: Pos,
  selected: usize,
  terrain: usize,
  vegetation: usize,
  water: usize,
  poi: usize,
  climate: usize,
  danger: i32,
  resources: i32,
}

impl HexEditor {
  pub fn new(cursor: Pos) -> Self {
    Self {
      cursor,
      selected: 0,
      terrain: 2,
      vegetation: 2,
      water: 2,
      poi: 0,
      climate: 0,
      danger: 1,
      resources: 1,
    }
  }

  pub fn selected(&self) -> usize {
    self.selected
  }

  pub fn move_cursor(&mut self, dx: i32, dy: i32) {
    self.cursor.0 += dx;
    self.cursor.1 += dy;
  }

  pub fn select_field(&mut self, delta: i32) {
    self.selected = (self.selected as i32 + delta).rem_euclid(FIELD_COUNT as i32) as usize;
  }

  pub fn cycle(&mut self, delta: i32) {
    match self.selected {
      0 => self.terrain = cycle(self.terrain, TERRAIN.len(), delta),
      1 => self.vegetation = cycle(self.vegetation, VEGETATION.len(), delta),
      2 => self.water = cycle(self.water, WATER.len(), delta),
      3 => self.poi = cycle(self.poi, POINTS_OF_INTEREST.len(), delta),
      4 => self.climate = cycle(self.climate, CLIMATE.len(), delta),
      5 => self.danger = (self.danger + delta).clamp(0, 10),
      6 => self.resources = (self.resources + delta).clamp(0, 10),
      _ => {}
    }
  }

  pub fn profile(&self) -> HexProfile {
    HexProfile {
      terrain: key(TERRAIN[self.terrain]),
      vegetation: key(VEGETATION[self.vegetation]),
      water: key(WATER[self.water]),
      point_of_interest: key(POINTS_OF_INTEREST[self.poi]),
      climate: key(CLIMATE[self.climate]),
      region: None,
      danger_level: self.danger,
      resource_richness: self.resources,
    }
  }

  pub fn lines(&self, visible: usize) -> Vec<String> {
    let profile = self.profile();
    let values = [
      format!("Terrain: {}", short_name(&profile.terrain)),
      format!("Vegetation: {}", short_name(&profile.vegetation)),
      format!("Water: {}", short_name(&profile.water)),
      format!("POI: {}", short_name(&profile.point_of_interest)),
      format!("Climate: {}", short_name(&profile.climate)),
      format!("Danger: {}", profile.danger_level),
      format!("Resources: {}", profile.resource_richness),
      "Region: None".to_string(),
    ];
    let start = self
      .selected
      .saturating_sub(visible.saturating_sub(1))
      .min(FIELD_COUNT.saturating_sub(visible));
    values
      .into_iter()
      .enumerate()
      .skip(start)
      .take(visible)
      .map(|(index, value)| {
        format!(
          "{} {}",
          if index == self.selected { ">" } else { " " },
          value
        )
      })
      .collect()
  }
}

fn key(value: &str) -> ContentKey {
  ContentKey::new(format!("core:{value}")).expect("fixed core content key is valid")
}

fn short_name(value: &ContentKey) -> &str {
  value.as_str().rsplit(':').next().unwrap_or(value.as_str())
}

fn cycle(current: usize, len: usize, delta: i32) -> usize {
  (current as i32 + delta).rem_euclid(len as i32) as usize
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn editor_cycles_profile_without_moving_cursor() {
    let mut editor = HexEditor::new((4, 5));
    editor.cycle(1);
    assert_eq!(editor.profile().terrain.as_str(), "core:swamp");
    assert_eq!(editor.cursor, (4, 5));
    editor.select_field(7);
    assert!(editor.lines(5).iter().any(|line| line.contains("Region")));
  }
}
