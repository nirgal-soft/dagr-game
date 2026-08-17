use dagr_lib::content::{ContentKey, MonsterDefinitionView};

#[derive(Clone, Debug)]
pub struct MonsterChoice {
  pub key: ContentKey,
  pub name: String,
  pub hit_dice: Vec<i32>,
  pub armor_class: i32,
}

impl From<MonsterDefinitionView> for MonsterChoice {
  fn from(definition: MonsterDefinitionView) -> Self {
    Self {
      key: definition.content_key,
      name: definition.name,
      hit_dice: definition.hit_dice,
      armor_class: definition.defense_value,
    }
  }
}

#[derive(Clone, Debug)]
pub struct MonsterPicker {
  query: String,
  choices: Vec<MonsterChoice>,
  selected: usize,
}

impl MonsterPicker {
  pub fn new(definitions: Vec<MonsterDefinitionView>) -> Self {
    Self {
      query: String::new(),
      choices: definitions.into_iter().map(Into::into).collect(),
      selected: 0,
    }
  }

  pub fn query(&self) -> &str {
    &self.query
  }

  pub fn input(&mut self, character: char) {
    if !character.is_control() {
      self.query.push(character);
      self.selected = 0;
    }
  }

  pub fn backspace(&mut self) {
    self.query.pop();
    self.selected = 0;
  }

  pub fn move_selection(&mut self, delta: i32) {
    let count = self.matches().len();
    if count == 0 {
      return;
    }
    self.selected = (self.selected as i32 + delta).rem_euclid(count as i32) as usize;
  }

  pub fn matches(&self) -> Vec<&MonsterChoice> {
    let mut matches = self
      .choices
      .iter()
      .filter_map(|choice| {
        fuzzy_score(&self.query, &format!("{} {}", choice.name, choice.key))
          .map(|score| (score, choice))
      })
      .collect::<Vec<_>>();
    matches.sort_by_key(|(score, choice)| (*score, choice.name.to_lowercase()));
    matches.into_iter().map(|(_, choice)| choice).collect()
  }

  pub fn selected(&self) -> Option<&MonsterChoice> {
    self.matches().get(self.selected).copied()
  }

  pub fn selected_index(&self) -> usize {
    self.selected
  }
}

fn fuzzy_score(query: &str, candidate: &str) -> Option<usize> {
  let query = query.to_lowercase();
  let candidate = candidate.to_lowercase();
  if query.is_empty() {
    return Some(0);
  }
  let mut chars = query.chars();
  let mut wanted = chars.next()?;
  for (index, character) in candidate.chars().enumerate() {
    if character == wanted {
      match chars.next() {
        Some(next) => wanted = next,
        None => return Some(index),
      }
    }
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  fn choice(name: &str, key: &str) -> MonsterChoice {
    MonsterChoice {
      name: name.into(),
      key: ContentKey::new(key).unwrap(),
      hit_dice: vec![1, 6, 0],
      armor_class: 3,
    }
  }

  #[test]
  fn lazy_matching_accepts_subsequences() {
    assert!(fuzzy_score("gbln", "Goblin core:goblin").is_some());
    assert!(fuzzy_score("hound", "Ash Hound core:ash_hound").is_some());
    assert!(fuzzy_score("dragon", "Goblin core:goblin").is_none());
  }

  #[test]
  fn selection_wraps_filtered_choices() {
    let mut picker = MonsterPicker {
      query: String::new(),
      choices: vec![choice("Boar", "core:boar"), choice("Goblin", "core:goblin")],
      selected: 0,
    };
    picker.move_selection(-1);
    assert_eq!(picker.selected().unwrap().name, "Goblin");
  }
}
