use std::sync::Arc;

use anyhow::Result;
use dagr_lib::{
  Engine,
  agency::{CreateFaction, CreateFactionMember, DefineNpcProfile, NpcMotivationInput},
  campaign::{CreateDanger, CreateFront, FrontType},
  characters::{BaseStats, CreateNpc},
  content::CharacterLevel,
};

pub struct DemoScenario {
  pub npc_id: dagr_lib::characters::CharacterId,
  pub target_id: dagr_lib::characters::CharacterId,
  pub faction_id: dagr_lib::agency::FactionId,
  pub danger_id: dagr_lib::campaign::DangerId,
}

pub async fn create(engine: Arc<Engine>) -> Result<DemoScenario> {
  let npc_id = create_npc(&engine, "Amleth the Cautious").await?;
  let target_id = create_npc(&engine, "Beatrice of the Old Road").await?;

  engine
    .agency()
    .define_npc_profile(DefineNpcProfile {
      character_id: npc_id,
      modifier: "cautious".into(),
      noun: "scholar".into(),
      bearing_demeanor: "skeptical".into(),
      bearing_focus: "the evidence".into(),
      current_focus: "the missing caravan".into(),
      conversation_mood: "guarded".into(),
      motivations: vec![
        NpcMotivationInput {
          verb: "discover".into(),
          noun: "truth".into(),
        },
        NpcMotivationInput {
          verb: "protect".into(),
          noun: "family".into(),
        },
        NpcMotivationInput {
          verb: "undermine".into(),
          noun: "authority".into(),
        },
      ],
    })
    .await?;

  let faction = engine
    .agency()
    .create_faction(CreateFaction {
      name: "The Brass Compact".into(),
      description: "Merchants and road wardens who control eastern trade.".into(),
      impulse: "To turn every obligation into leverage".into(),
      goal: "Control passage through the valley".into(),
      members: vec![CreateFactionMember {
        character_id: npc_id,
        role: "road marshal".into(),
      }],
    })
    .await?
    .faction;
  let front = engine
    .campaign()
    .create_front(CreateFront {
      name: "The Closed Road".into(),
      description: "The Compact turns commerce into a weapon.".into(),
      front_type: FrontType::Campaign,
      stakes: vec!["Who will break the Compact's hold on the valley?".into()],
      cast: Vec::new(),
      dangers: vec![CreateDanger {
        faction_id: Some(faction.faction_id),
        category: "Ambitious Organization".into(),
        name: "The Brass Compact".into(),
        impulse: "To maintain control through debt and force".into(),
        impending_doom: "Tyranny".into(),
        grim_portents: vec![
          "The eastern toll doubles".into(),
          "Independent caravans disappear".into(),
          "The road closes to all but Compact allies".into(),
        ],
        moves: vec!["Call in a debt".into(), "Seize a caravan".into()],
      }],
    })
    .await?
    .front;
  Ok(DemoScenario {
    npc_id,
    target_id,
    faction_id: faction.faction_id,
    danger_id: front.dangers[0].danger_id,
  })
}

async fn create_npc(
  engine: &Engine,
  name: &str,
) -> Result<dagr_lib::characters::CharacterId> {
  Ok(
    engine
      .characters()
      .create_npc(CreateNpc {
        name: name.to_string(),
        level: CharacterLevel::new(1)?,
        base_stats: BaseStats::default(),
      })
      .await?
      .character
      .id,
  )
}

#[cfg(test)]
mod tests {
  use std::{fs, time::{SystemTime, UNIX_EPOCH}};

  use dagr_lib::{EngineConfig, NewWorld};

  use super::*;

  #[tokio::test]
  async fn demo_preserves_the_authored_fixture() {
    let root = std::env::temp_dir().join(format!(
      "dagr-debug-scenario-{}-{}",
      std::process::id(),
      SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let engine = Arc::new(
      Engine::open(EngineConfig {
        world_path: root.join("world.sqlite3"),
        core_content_path: crate::startup::core_content_path(),
        new_world: Some(NewWorld { seed: 0 }),
      })
      .await
      .unwrap(),
    );

    let scenario = create(engine.clone()).await.unwrap();
    assert_eq!(
      engine
        .characters()
        .character(scenario.npc_id)
        .await
        .unwrap()
        .name,
      "Amleth the Cautious"
    );
    assert_eq!(
      engine
        .characters()
        .character(scenario.target_id)
        .await
        .unwrap()
        .name,
      "Beatrice of the Old Road"
    );
    let profile = engine.agency().npc_profile(scenario.npc_id).await.unwrap();
    assert_eq!(
      profile
        .motivations
        .iter()
        .map(|motivation| (motivation.verb.as_str(), motivation.noun.as_str()))
        .collect::<Vec<_>>(),
      [
        ("discover", "truth"),
        ("protect", "family"),
        ("undermine", "authority")
      ]
    );
    let faction = engine.agency().faction(scenario.faction_id).await.unwrap();
    assert_eq!(faction.name, "The Brass Compact");
    assert_eq!(faction.members[0].character_id, scenario.npc_id);
    assert_eq!(faction.members[0].role, "road marshal");
    let front_id = engine.campaign().active_fronts().await.unwrap()[0].front_id;
    let front = engine.campaign().front(front_id).await.unwrap();
    assert_eq!(front.front_type, FrontType::Campaign);
    assert_eq!(front.stakes[0].description, "Who will break the Compact's hold on the valley?");
    assert_eq!(front.dangers[0].danger_id, scenario.danger_id);
    assert_eq!(front.dangers[0].name, "The Brass Compact");
    assert_eq!(
      front.dangers[0]
        .grim_portents
        .iter()
        .map(|portent| portent.description.as_str())
        .collect::<Vec<_>>(),
      [
        "The eastern toll doubles",
        "Independent caravans disappear",
        "The road closes to all but Compact allies"
      ]
    );

    drop(engine);
    fs::remove_dir_all(root).unwrap();
  }
}
