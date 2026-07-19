use std::sync::Arc;

use anyhow::Result;
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
      "1" => {
        if let Err(error) = list_catalog(&router){
          show_recoverable_error(&error)?;
        }
      }
      "2" => {
        if let Some(actor) = prompt_actor()?{
          match contexts.build(actor).await{
            Ok(context) => print_json("Actor context", &context)?,
            Err(error) => println!("Unable to build context: {error:#}"),
          }
          pause()?;
        }
      }
      "3" => {
        if let Err(error) = invoke_tool(&router, &contexts, pool.as_ref()).await{
          show_recoverable_error(&error)?;
        }
      }
      "4" => {
        if let Err(error) = view_events(pool.as_ref()).await{
          show_recoverable_error(&error)?;
        }
      }
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
  for (index, tool) in definitions.iter().enumerate(){
    println!("  {}. {} - {}", index + 1, tool.name, tool.description);
    println!("     {}", serde_json::to_string(&tool.input_schema)?);
  }
  let Some(tool_index) = prompt_tool(&definitions)? else{return Ok(())};
  let tool = &definitions[tool_index];
  let Some(input) = prompt_arguments(tool)? else{return Ok(())};
  let invocation = ToolInvocation{
    actor,
    call: ToolCall{
      id: format!("debug-{}", std::process::id()),
      name: tool.name.to_string(),
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
  loop{
    println!();
    println!("Actor scope:");
    println!("  1. GM");
    println!("  2. NPC");
    println!("  3. Faction");
    println!("  b. Back");
    match prompt("Scope: ")?.trim().to_ascii_lowercase().as_str(){
      "1" | "g" | "gm" => return Ok(Some(ActorScope::Gm)),
      "2" | "n" | "npc" => {
        let Some(id) = prompt_positive_id("NPC character ID")? else{continue};
        return Ok(Some(ActorScope::Npc(CharacterId::new(id)?)))
      }
      "3" | "f" | "faction" => {
        let Some(id) = prompt_positive_id("Faction ID")? else{continue};
        return Ok(Some(ActorScope::Faction(FactionId::new(id)?)))
      }
      "b" | "back" | "cancel" => return Ok(None),
      _ => println!("Invalid scope. Choose 1, 2, 3, or b."),
    }
  }
}

fn prompt_positive_id(label: &str) -> Result<Option<i32>>{
  loop{
    let input = prompt(&format!("{label} (or b to cancel): "))?;
    if matches!(input.trim().to_ascii_lowercase().as_str(), "b" | "back" | "cancel"){
      return Ok(None)
    }
    match parse_positive_id(&input){
      Ok(id) => return Ok(Some(id)),
      Err(message) => println!("{message}"),
    }
  }
}

fn parse_positive_id(input: &str) -> std::result::Result<i32, &'static str>{
  match input.trim().parse::<i32>(){
    Ok(id) if id > 0 => Ok(id),
    Ok(_) => Err("ID must be greater than zero."),
    Err(_) => Err("ID must be a positive integer."),
  }
}

fn prompt_tool(definitions: &[dagr_lib::agency::ToolDefinition]) -> Result<Option<usize>>{
  loop{
    let input = prompt("Tool number or name (b to cancel): ")?;
    let normalized = input.trim();
    if matches!(normalized.to_ascii_lowercase().as_str(), "b" | "back" | "cancel"){
      return Ok(None)
    }
    if let Ok(index) = normalized.parse::<usize>(){
      if let Some(index) = index.checked_sub(1).filter(|index| *index < definitions.len()){
        return Ok(Some(index))
      }
    }
    if let Some(index) = definitions.iter().position(|tool| tool.name == normalized){
      return Ok(Some(index))
    }
    println!("Unknown tool. Choose one of the displayed numbers or names.");
  }
}

fn prompt_arguments(tool: &dagr_lib::agency::ToolDefinition) -> Result<Option<Value>>{
  loop{
    println!("Arguments for {}:", tool.name);
    println!("  1. Guided field entry");
    println!("  2. Raw JSON object");
    println!("  b. Cancel");
    match prompt("Entry mode: ")?.trim().to_ascii_lowercase().as_str(){
      "1" | "g" | "guided" => return prompt_guided_arguments(&tool.input_schema),
      "2" | "j" | "json" => return prompt_json_arguments(),
      "b" | "back" | "cancel" => return Ok(None),
      _ => println!("Invalid mode. Choose 1, 2, or b."),
    }
  }
}

fn prompt_guided_arguments(schema: &Value) -> Result<Option<Value>>{
  let Some(properties) = schema.get("properties").and_then(Value::as_object) else{
    println!("This tool has no guided schema; use raw JSON instead.");
    return prompt_json_arguments()
  };
  let required = schema.get("required").and_then(Value::as_array);
  let mut result = serde_json::Map::new();
  for (name, property) in properties{
    let is_required = required.is_some_and(|fields| fields.iter().any(|field| field == name));
    let kind = property.get("type").and_then(Value::as_str).unwrap_or("value");
    let description = property.get("description").and_then(Value::as_str).unwrap_or("");
    loop{
      println!("  {name} ({kind}){}: {description}", if is_required{" [required]"}else{""});
      let input = prompt("    value (:back to cancel): ")?;
      if input.trim().eq_ignore_ascii_case(":back"){
        return Ok(None)
      }
      if input.trim().is_empty() && !is_required{
        break
      }
      match parse_schema_value(kind, &input){
        Ok(value) => {
          result.insert(name.clone(), value);
          break
        }
        Err(message) => println!("    {message}"),
      }
    }
  }
  Ok(Some(Value::Object(result)))
}

fn parse_schema_value(kind: &str, input: &str) -> std::result::Result<Value, String>{
  match kind{
    "string" if input.trim().is_empty() => Err("Value cannot be empty.".to_string()),
    "string" => Ok(Value::String(input.to_string())),
    "integer" => input.trim().parse::<i64>().map(Value::from)
      .map_err(|_| "Enter a whole number.".to_string()),
    "number" => input.trim().parse::<f64>().map(Value::from)
      .map_err(|_| "Enter a number.".to_string()),
    "boolean" => match input.trim().to_ascii_lowercase().as_str(){
      "true" | "yes" | "y" | "1" => Ok(Value::Bool(true)),
      "false" | "no" | "n" | "0" => Ok(Value::Bool(false)),
      _ => Err("Enter true or false.".to_string()),
    },
    _ => serde_json::from_str(input).map_err(|error| format!("Enter valid JSON: {error}")),
  }
}

fn prompt_json_arguments() -> Result<Option<Value>>{
  loop{
    let source = prompt("JSON arguments (:back to cancel): ")?;
    if source.trim().eq_ignore_ascii_case(":back"){
      return Ok(None)
    }
    let source = if source.trim().is_empty(){"{}"}else{source.trim()};
    match serde_json::from_str(source){
      Ok(value @ Value::Object(_)) => return Ok(Some(value)),
      Ok(_) => println!("Arguments must be a JSON object, such as {{\"reason\":\"test\"}}."),
      Err(error) => println!("Invalid JSON: {error}. Correct it or enter :back."),
    }
  }
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

fn show_recoverable_error(error: &anyhow::Error) -> Result<()>{
  println!("Unable to complete that operation: {error:#}");
  println!("The debug console is still available; adjust the input and try again.");
  pause()
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
  fn validates_positive_actor_ids(){
    assert_eq!(parse_positive_id("42"), Ok(42));
    assert!(parse_positive_id("0").is_err());
    assert!(parse_positive_id("-1").is_err());
    assert!(parse_positive_id("not-an-id").is_err());
  }

  #[test]
  fn guided_values_reject_invalid_input(){
    assert_eq!(parse_schema_value("integer", "12").unwrap(), Value::from(12));
    assert!(parse_schema_value("integer", "twelve").is_err());
    assert!(parse_schema_value("string", "").is_err());
    assert_eq!(parse_schema_value("boolean", "yes").unwrap(), Value::Bool(true));
  }
}
