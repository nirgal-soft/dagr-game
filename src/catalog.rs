use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use dagr_lib::characters::CharacterId;
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;

const CATALOG_VERSION: u32 = 1;
const CATALOG_FILE: &str = "catalog.json";
const WORLDS_DIR: &str = "worlds";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Catalog {
  pub version: u32,
  pub local_player: LocalPlayer,
  pub worlds: BTreeMap<String, WorldRecord>,
}

impl Default for Catalog {
  fn default() -> Self {
    Self {
      version: CATALOG_VERSION,
      local_player: LocalPlayer::default(),
      worlds: BTreeMap::new(),
    }
  }
}

impl Catalog {
  pub fn world(&self, world_id: &str) -> Result<&WorldRecord> {
    self
      .worlds
      .get(world_id)
      .with_context(|| format!("managed world '{world_id}' is not in the catalog"))
  }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LocalPlayer {
  pub world_bindings: BTreeMap<String, WorldBinding>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorldBinding {
  pub roster: Vec<CharacterId>,
  pub active_character: CharacterId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorldRecord {
  pub display_name: String,
  pub relative_sqlite_path: PathBuf,
  pub setup_state: SetupState,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupState {
  Creating,
  Ready,
}

pub struct CatalogStore {
  root: PathBuf,
}

impl CatalogStore {
  pub fn discover() -> Result<Self> {
    let base =
      BaseDirs::new().context("platform application-data directory is unavailable")?;
    Ok(Self::at(base.data_dir().join("dagr")))
  }

  pub fn at(root: PathBuf) -> Self {
    Self { root }
  }

  pub fn load(&self) -> Result<Catalog> {
    fs::create_dir_all(self.worlds_dir()).with_context(|| {
      format!(
        "create managed worlds directory {}",
        self.worlds_dir().display()
      )
    })?;
    let path = self.catalog_path();
    if !path.exists() {
      let catalog = Catalog::default();
      self.save(&catalog)?;
      return Ok(catalog);
    }
    let reader = BufReader::new(
      File::open(&path).with_context(|| format!("open managed catalog {}", path.display()))?,
    );
    let catalog: Catalog = serde_json::from_reader(reader)
      .with_context(|| format!("parse managed catalog {}", path.display()))?;
    validate(&catalog)?;
    Ok(catalog)
  }

  pub fn save(&self, catalog: &Catalog) -> Result<()> {
    validate(catalog)?;
    fs::create_dir_all(&self.root)
      .with_context(|| format!("create catalog directory {}", self.root.display()))?;
    fs::create_dir_all(self.worlds_dir()).with_context(|| {
      format!(
        "create managed worlds directory {}",
        self.worlds_dir().display()
      )
    })?;
    let destination = self.catalog_path();
    let temporary = self
      .root
      .join(format!(".{CATALOG_FILE}.{}.tmp", std::process::id()));
    {
      let file = File::create(&temporary)
        .with_context(|| format!("create temporary catalog {}", temporary.display()))?;
      let mut writer = BufWriter::new(file);
      serde_json::to_writer_pretty(&mut writer, catalog).context("serialize managed catalog")?;
      writer.write_all(b"\n")?;
      writer.flush()?;
      writer.get_ref().sync_all()?;
    }
    fs::rename(&temporary, &destination).with_context(|| {
      format!(
        "atomically replace managed catalog {} with {}",
        destination.display(),
        temporary.display()
      )
    })?;
    File::open(&self.root)?.sync_all()?;
    Ok(())
  }

  pub fn begin_world(&self, catalog: &mut Catalog, display_name: String) -> Result<String> {
    let display_name = display_name.trim().to_string();
    if display_name.is_empty() {
      bail!("world display name must not be blank");
    }
    if catalog
      .worlds
      .values()
      .any(|world| display_name_key(&world.display_name) == display_name_key(&display_name))
    {
      bail!("a world named '{display_name}' already exists");
    }
    let id = next_world_id(catalog);
    catalog.worlds.insert(
      id.clone(),
      WorldRecord {
        display_name,
        relative_sqlite_path: PathBuf::from(WORLDS_DIR).join(format!("{id}.sqlite3")),
        setup_state: SetupState::Creating,
      },
    );
    self.save(catalog)?;
    Ok(id)
  }

  pub fn finish_world(
    &self,
    catalog: &mut Catalog,
    world_id: &str,
    active_character: CharacterId,
  ) -> Result<()> {
    let world = catalog
      .worlds
      .get_mut(world_id)
      .with_context(|| format!("managed world '{world_id}' disappeared during setup"))?;
    world.setup_state = SetupState::Ready;
    catalog.local_player.world_bindings.insert(
      world_id.to_string(),
      WorldBinding {
        roster: vec![active_character],
        active_character,
      },
    );
    self.save(catalog)
  }

  pub fn world_path(&self, record: &WorldRecord) -> PathBuf {
    self.root.join(&record.relative_sqlite_path)
  }

  pub fn ready_worlds<'a>(&self, catalog: &'a Catalog) -> Vec<(&'a str, &'a WorldRecord)> {
    let mut worlds = catalog
      .worlds
      .iter()
      .filter(|(_, world)| world.setup_state == SetupState::Ready)
      .map(|(id, world)| (id.as_str(), world))
      .collect::<Vec<_>>();
    worlds.sort_by(|(left_id, left), (right_id, right)| {
      display_name_key(&left.display_name)
        .cmp(&display_name_key(&right.display_name))
        .then_with(|| left_id.cmp(right_id))
    });
    worlds
  }

  fn catalog_path(&self) -> PathBuf {
    self.root.join(CATALOG_FILE)
  }

  fn worlds_dir(&self) -> PathBuf {
    self.root.join(WORLDS_DIR)
  }
}

fn next_world_id(catalog: &Catalog) -> String {
  let mut sequence = 1_u64;
  loop {
    let candidate = format!("world-{sequence:04}");
    if !catalog.worlds.contains_key(&candidate) {
      return candidate;
    }
    sequence += 1;
  }
}

pub(crate) fn display_name_key(display_name: &str) -> String {
  display_name
    .trim()
    .chars()
    .nfkc()
    .case_fold()
    .nfkc()
    .collect()
}

fn validate(catalog: &Catalog) -> Result<()> {
  if catalog.version != CATALOG_VERSION {
    bail!(
      "managed catalog version {} is unsupported; expected {CATALOG_VERSION}",
      catalog.version
    );
  }
  let mut names = HashSet::new();
  for (id, world) in &catalog.worlds {
    if world.display_name.trim().is_empty() {
      bail!("managed world '{id}' has a blank display name");
    }
    if !names.insert(display_name_key(&world.display_name)) {
      bail!("managed world display names must be unique case-insensitively");
    }
    validate_world_path(id, &world.relative_sqlite_path)?;
    if world.setup_state == SetupState::Ready {
      let binding = catalog
        .local_player
        .world_bindings
        .get(id)
        .with_context(|| format!("ready managed world '{id}' has no local-player binding"))?;
      if binding.roster.is_empty() || !binding.roster.contains(&binding.active_character) {
        bail!("managed world '{id}' has an invalid active-character binding");
      }
    }
  }
  for id in catalog.local_player.world_bindings.keys() {
    if !catalog.worlds.contains_key(id) {
      bail!("local-player binding references unknown managed world '{id}'");
    }
  }
  Ok(())
}

fn validate_world_path(id: &str, path: &Path) -> Result<()> {
  let expected = PathBuf::from(WORLDS_DIR).join(format!("{id}.sqlite3"));
  if path != expected
    || path.is_absolute()
    || path
      .components()
      .any(|component| !matches!(component, Component::Normal(_)))
  {
    bail!("managed world '{id}' has an invalid SQLite path");
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn temporary_store() -> CatalogStore {
    CatalogStore::at(std::env::temp_dir().join(format!(
      "dagr-game-catalog-{}-{}",
      std::process::id(),
      std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
    )))
  }

  #[test]
  fn catalog_lifecycle_is_atomic_and_ready_worlds_are_sorted() {
    let store = temporary_store();
    let mut catalog = store.load().unwrap();
    let zeta = store.begin_world(&mut catalog, "Zeta".into()).unwrap();
    let alpha = store.begin_world(&mut catalog, "alpha".into()).unwrap();
    store
      .finish_world(&mut catalog, &zeta, CharacterId::new(7).unwrap())
      .unwrap();
    store
      .finish_world(&mut catalog, &alpha, CharacterId::new(8).unwrap())
      .unwrap();

    let reopened = store.load().unwrap();
    let names = store
      .ready_worlds(&reopened)
      .into_iter()
      .map(|(_, world)| world.display_name.as_str())
      .collect::<Vec<_>>();
    assert_eq!(names, ["alpha", "Zeta"]);
    assert_eq!(
      reopened.local_player.world_bindings[&zeta].active_character,
      CharacterId::new(7).unwrap()
    );
    fs::remove_dir_all(&store.root).unwrap();
  }

  #[test]
  fn names_are_unique_case_insensitively_and_creating_worlds_are_hidden() {
    let store = temporary_store();
    let mut catalog = store.load().unwrap();
    store
      .begin_world(&mut catalog, "Grey March".into())
      .unwrap();
    assert!(store.ready_worlds(&catalog).is_empty());
    assert!(
      store
        .begin_world(&mut catalog, "grey march".into())
        .is_err()
    );
    store.begin_world(&mut catalog, "Éire".into()).unwrap();
    assert!(store.begin_world(&mut catalog, "éire".into()).is_err());
    store.begin_world(&mut catalog, "Straße".into()).unwrap();
    assert!(store.begin_world(&mut catalog, "STRASSE".into()).is_err());
    store.begin_world(&mut catalog, "Åland".into()).unwrap();
    assert!(
      store
        .begin_world(&mut catalog, "A\u{30a}land".into())
        .is_err()
    );
    fs::remove_dir_all(&store.root).unwrap();
  }
}
