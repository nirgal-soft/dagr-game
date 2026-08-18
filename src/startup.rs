//! Managed-world selection and Engine startup.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use dagr_lib::{
  Engine, EngineConfig, NewWorld, SpawnPlayer,
  characters::{CharacterId, CreatePlayer},
  content::{CharacterLevel, ContentKey},
  world::GenerateHex,
};

use crate::catalog::{Catalog, CatalogStore};
use crate::{gateway, menu};

pub struct OpenedWorld {
  pub engine: Arc<Engine>,
  pub active_player: CharacterId,
  pub display_name: String,
  pub model_runtimes: gateway::ModelRuntimes,
}

pub async fn select() -> Result<Option<OpenedWorld>> {
  let store = CatalogStore::discover()?;
  let mut catalog = store.load()?;
  let ready_worlds = store.ready_worlds(&catalog);
  let reserved_names = catalog
    .worlds
    .values()
    .map(|world| world.display_name.as_str())
    .collect::<Vec<_>>();
  let selection = menu::show_world_picker(&ready_worlds, &reserved_names, 0)?;
  match selection {
    menu::WorldSelection::Existing(world_id) => {
      let display_name = catalog.world(&world_id)?.display_name.clone();
      let (engine, active_player, model_runtimes) = open_world(&store, &catalog, &world_id).await?;
      Ok(Some(OpenedWorld {
        engine,
        active_player,
        display_name,
        model_runtimes,
      }))
    }
    menu::WorldSelection::Create { display_name, seed } => {
      let (engine, active_player, model_runtimes) =
        create_world(&store, &mut catalog, display_name.clone(), seed).await?;
      Ok(Some(OpenedWorld {
        engine,
        active_player,
        display_name,
        model_runtimes,
      }))
    }
    menu::WorldSelection::Quit => Ok(None),
  }
}

async fn open_world(
  store: &CatalogStore,
  catalog: &Catalog,
  world_id: &str,
) -> Result<(Arc<Engine>, CharacterId, gateway::ModelRuntimes)> {
  let record = catalog.world(world_id)?;
  let binding = catalog
    .local_player
    .world_bindings
    .get(world_id)
    .with_context(|| format!("managed world '{world_id}' has no player binding"))?;
  let (engine, model_runtimes) = open_engine(EngineConfig {
    world_path: store.world_path(record),
    core_content_path: core_content_path(),
    new_world: None,
  })
  .await?;
  for character_id in &binding.roster {
    engine
      .characters()
      .character(*character_id)
      .await
      .with_context(|| {
        format!("managed world '{world_id}' roster references missing Character {character_id}")
      })?;
  }
  Ok((Arc::new(engine), binding.active_character, model_runtimes))
}

async fn create_world(
  store: &CatalogStore,
  catalog: &mut Catalog,
  display_name: String,
  seed: u64,
) -> Result<(Arc<Engine>, CharacterId, gateway::ModelRuntimes)> {
  let world_id = store.begin_world(catalog, display_name)?;
  let record = catalog
    .worlds
    .get(&world_id)
    .context("new managed world has no catalog record")?;
  let (engine, model_runtimes) = open_engine(EngineConfig {
    world_path: store.world_path(record),
    core_content_path: core_content_path(),
    new_world: Some(NewWorld { seed }),
  })
  .await?;
  let engine = Arc::new(engine);
  let origin = engine
    .world()
    .generate_hex(GenerateHex::generated(0, 0, None))
    .await?
    .hex;
  let player = engine
    .spawn_player(SpawnPlayer {
      character: CreatePlayer {
        name: "Adventurer".to_string(),
        class: ContentKey::new("core:strong")?,
        level: CharacterLevel::new(1)?,
        seed,
      },
      location: origin.location_id,
      x: 0,
      y: 0,
    })
    .await?
    .character
    .id;
  store.finish_world(catalog, &world_id, player)?;
  Ok((engine, player, model_runtimes))
}

pub(crate) fn core_content_path() -> PathBuf {
  std::env::var_os("DAGR_CORE_CONTENT_PATH")
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dagr/content/core"))
}

async fn open_engine(config: EngineConfig) -> Result<(Engine, gateway::ModelRuntimes)> {
  let runtimes = gateway::ModelRuntimes::from_env()?;
  let engine = Engine::open_with_agent_runtime(config, runtimes.agent.clone()).await?;
  Ok((engine, runtimes))
}
