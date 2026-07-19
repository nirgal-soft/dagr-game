use std::sync::Arc;

use anyhow::{Context, Result};
use dagr_lib::{
  agency::{
    events::{load_actor_events, ActorScope},
    factions::{CreateFaction, CreateFactionMember, FactionService},
    npcs::{CreateNpcProfile, Motivation, NpcAgencyService},
    runner::ActorContextBuilder,
    ToolCall, ToolInvocation, ToolRouter,
  },
  archetypes::characters::character::NPCTypeData,
  campaign::fronts::{CreateDanger, CreateFront, FrontService, FrontType},
  components::{
    characters::{character::CharacterData, character_types::CharacterType},
    stats::base_stats::BaseStatsData,
  },
  ems::component::Component,
  factories::characters::character::CharacterFactory,
  ids::{CharacterId, FactionId},
};
use serde_json::Value;
use sqlx::PgPool;

use crate::menu::prompt;

pub async fn run(pool: Arc<PgPool>) -> Result<()>{
  let router = ToolRouter::new(pool.clone());
  let contexts = ActorContextBuilder::new(pool.clone());
  loop{
    print!("\x1b[2J\x1b[H");
    println!("DAGR Engine Debug Tools");
    println!("=======================");
    println!();
    println!("1. List complete tool catalog");
    println!("2. Inspect actor context");
    println!("3. Invoke a scoped tool");
    println!("4. View recent actor events");
    println!("5. Create demo NPC/Faction/Front scenario");
    println!("b. Back to main menu");
    println!();
    match prompt("Select: ")?.trim().to_ascii_lowercase().as_str(){
      "1" => list_catalog(&router)?,
      "2" => {
        if let Some(actor) = prompt_actor()?{
          match contexts.build(actor).await{
            Ok(context) => print_json("Actor context", &context)?,
            Err(error) => println!("Unable to build context: {error:#}"),
          }
          pause()?;
        }
      }
      "3" => invoke_tool(&router, &contexts, pool.as_ref()).await?,
      "4" => view_events(pool.as_ref()).await?,
      "5" => {
        match create_demo_scenario(pool.clone()).await{
          Ok(scenario) => {
            println!("Created demo scenario:");
            println!("  NPC actor:       {}", scenario.npc_id);
            println!("  Relationship target: {}", scenario.target_id);
            println!("  Faction actor:   {}", scenario.faction_id);
            println!("  Front danger:    {}", scenario.danger_id);
            println!();
            println!("Use those IDs with the scoped tools.");
          }
          Err(error) => println!("Failed to create demo scenario: {error:#}"),
        }
        pause()?;
      }
      "b" | "back" | "q" => return Ok(()),
      _ => {
        println!("Unknown selection.");
        pause()?;
      }
    }
  }
}

fn list_catalog(router: &ToolRouter) -> Result<()>{
  println!();
  for tool in router.catalog(){
    println!("{}", tool.name);
    println!("  {}", tool.description);
    println!("  schema: {}", serde_json::to_string_pretty(&tool.input_schema)?);
  }
  pause()
}

async fn invoke_tool(
  router: &ToolRouter,
  contexts: &ActorContextBuilder,
  pool: &PgPool,
) -> Result<()>{
  let Some(actor) = prompt_actor()? else{return Ok(())};
  let definitions = router.definitions(actor);
  if definitions.is_empty(){
    println!("No tools are available for this actor scope.");
    pause()?;
    return Ok(())
  }
  println!("Available tools:");
  for tool in &definitions{
    println!("  {} - {}", tool.name, tool.description);
    println!("    {}", serde_json::to_string(&tool.input_schema)?);
  }
  let name = prompt("Tool name: ")?;
  let input_source = prompt("JSON arguments: ")?;
  let input: Value = serde_json::from_str(if input_source.trim().is_empty(){"{}"}else{&input_source})
    .context("invalid JSON arguments")?;
  let invocation = ToolInvocation{
    actor,
    call: ToolCall{
      id: format!("debug-{}", std::process::id()),
      name,
      input,
    },
  };
  let result = router.invoke(&invocation).await;
  print_json("Tool result", &serde_json::to_value(&result)?)?;
  if result.success{
    match contexts.build(actor).await{
      Ok(context) => print_json("Refreshed actor context", &context)?,
      Err(error) => println!("Unable to refresh actor context: {error:#}"),
    }
    let events = load_actor_events(pool, actor).await?;
    print_json("Persisted consequences", &serde_json::to_value(events)?)?;
  }
  pause()
}

async fn view_events(pool: &PgPool) -> Result<()>{
  let Some(actor) = prompt_actor()? else{return Ok(())};
  let events = load_actor_events(pool, actor).await?;
  print_json("Recent actor events", &serde_json::to_value(events)?)?;
  pause()
}

fn prompt_actor() -> Result<Option<ActorScope>>{
  println!();
  println!("Actor scope: g = GM, n = NPC, f = Faction, b = back");
  match prompt("Scope: ")?.trim().to_ascii_lowercase().as_str(){
    "g" | "gm" => Ok(Some(ActorScope::Gm)),
    "n" | "npc" => {
      let id = parse_positive_id("NPC character ID: ")?;
      Ok(Some(ActorScope::Npc(CharacterId::new(id)?)))
    }
    "f" | "faction" => {
      let id = parse_positive_id("Faction ID: ")?;
      Ok(Some(ActorScope::Faction(FactionId::new(id)?)))
    }
    "b" | "back" => Ok(None),
    other => anyhow::bail!("unknown actor scope '{other}'"),
  }
}

fn parse_positive_id(label: &str) -> Result<i32>{
  prompt(label)?.trim().parse::<i32>().context("ID must be a positive integer")
}

fn print_json(label: &str, value: &Value) -> Result<()>{
  println!();
  println!("{label}:");
  println!("{}", serde_json::to_string_pretty(value)?);
  Ok(())
}

fn pause() -> Result<()>{
  let _ = prompt("\nPress Enter to continue...")?;
  Ok(())
}

struct DemoScenario{
  npc_id: CharacterId,
  target_id: CharacterId,
  faction_id: FactionId,
  danger_id: dagr_lib::ids::DangerId,
}

async fn create_demo_scenario(pool: Arc<PgPool>) -> Result<DemoScenario>{
  let npc_id = create_npc(&pool, "Amleth the Cautious").await?;
  let target_id = create_npc(&pool, "Beatrice of the Old Road").await?;
  NpcAgencyService::new(pool.clone()).create_profile(CreateNpcProfile{
    character_id: npc_id,
    modifier: "cautious".to_string(),
    noun: "scholar".to_string(),
    bearing_demeanor: "skeptical".to_string(),
    bearing_focus: "the evidence".to_string(),
    current_focus: "the missing caravan".to_string(),
    conversation_mood: "guarded".to_string(),
    motivations: vec![
      Motivation{position: 1, verb: "discover".to_string(), noun: "truth".to_string()},
      Motivation{position: 2, verb: "protect".to_string(), noun: "family".to_string()},
      Motivation{position: 3, verb: "undermine".to_string(), noun: "authority".to_string()},
    ],
  }).await?;
  let faction = FactionService::new(pool.clone()).create(CreateFaction{
    name: "The Brass Compact".to_string(),
    description: "Merchants and road wardens who control eastern trade.".to_string(),
    impulse: "To turn every obligation into leverage".to_string(),
    goal: "Control passage through the valley".to_string(),
    members: vec![CreateFactionMember{character_id: npc_id, role: "road marshal".to_string()}],
  }).await?;
  let front = FrontService::new(pool).create(CreateFront{
    name: "The Closed Road".to_string(),
    description: "The Compact turns commerce into a weapon.".to_string(),
    front_type: FrontType::Campaign,
    stakes: vec!["Who will break the Compact's hold on the valley?".to_string()],
    cast: Vec::new(),
    dangers: vec![CreateDanger{
      faction_id: Some(faction.faction.id),
      category: "Ambitious Organization".to_string(),
      name: "The Brass Compact".to_string(),
      impulse: "To maintain control through debt and force".to_string(),
      impending_doom: "Tyranny".to_string(),
      grim_portents: vec![
        "The eastern toll doubles".to_string(),
        "Independent caravans disappear".to_string(),
        "The road closes to all but Compact allies".to_string(),
      ],
      moves: vec!["Call in a debt".to_string(), "Seize a caravan".to_string()],
    }],
  }).await?;
  Ok(DemoScenario{
    npc_id,
    target_id,
    faction_id: faction.faction.id,
    danger_id: front.dangers[0].danger.id,
  })
}

async fn create_npc(pool: &PgPool, name: &str) -> Result<CharacterId>{
  let archetype = CharacterFactory::new(Arc::new(pool.clone())).create_npc(NPCTypeData{
    character: CharacterData::new(name.to_string(), CharacterType::NPC),
    base_stats: BaseStatsData::default(),
  }).await?;
  Ok(archetype.character.get().get_id()?)
}

#[cfg(test)]
mod tests{
  use super::*;

  #[test]
  fn rejects_non_numeric_ids(){
    assert!("not-an-id".parse::<i32>().is_err());
  }
}
