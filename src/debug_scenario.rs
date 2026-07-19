use std::sync::Arc;

use anyhow::Result;
use dagr_lib::{
  agency::{
    factions::{CreateFaction, CreateFactionMember, FactionService},
    npcs::{CreateNpcProfile, Motivation, NpcAgencyService},
  },
  archetypes::characters::character::NPCTypeData,
  campaign::fronts::{CreateDanger, CreateFront, FrontService, FrontType},
  components::{
    characters::{character::CharacterData, character_types::CharacterType},
    stats::base_stats::BaseStatsData,
  },
  ems::component::Component,
  factories::characters::character::CharacterFactory,
  ids::{CharacterId, DangerId, FactionId},
};
use sqlx::PgPool;

pub struct DemoScenario {
  pub npc_id: CharacterId,
  pub target_id: CharacterId,
  pub faction_id: FactionId,
  pub danger_id: DangerId,
}

pub async fn create(pool: Arc<PgPool>) -> Result<DemoScenario> {
  let npc_id = create_npc(&pool, "Amleth the Cautious").await?;
  let target_id = create_npc(&pool, "Beatrice of the Old Road").await?;
  NpcAgencyService::new(pool.clone())
    .create_profile(CreateNpcProfile {
      character_id: npc_id,
      modifier: "cautious".into(),
      noun: "scholar".into(),
      bearing_demeanor: "skeptical".into(),
      bearing_focus: "the evidence".into(),
      current_focus: "the missing caravan".into(),
      conversation_mood: "guarded".into(),
      motivations: vec![
        Motivation {
          position: 1,
          verb: "discover".into(),
          noun: "truth".into(),
        },
        Motivation {
          position: 2,
          verb: "protect".into(),
          noun: "family".into(),
        },
        Motivation {
          position: 3,
          verb: "undermine".into(),
          noun: "authority".into(),
        },
      ],
    })
    .await?;
  let faction = FactionService::new(pool.clone())
    .create(CreateFaction {
      name: "The Brass Compact".into(),
      description: "Merchants and road wardens who control eastern trade.".into(),
      impulse: "To turn every obligation into leverage".into(),
      goal: "Control passage through the valley".into(),
      members: vec![CreateFactionMember {
        character_id: npc_id,
        role: "road marshal".into(),
      }],
    })
    .await?;
  let front = FrontService::new(pool)
    .create(CreateFront {
      name: "The Closed Road".into(),
      description: "The Compact turns commerce into a weapon.".into(),
      front_type: FrontType::Campaign,
      stakes: vec!["Who will break the Compact's hold on the valley?".into()],
      cast: Vec::new(),
      dangers: vec![CreateDanger {
        faction_id: Some(faction.faction.id),
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
    .await?;
  Ok(DemoScenario {
    npc_id,
    target_id,
    faction_id: faction.faction.id,
    danger_id: front.dangers[0].danger.id,
  })
}

async fn create_npc(pool: &PgPool, name: &str) -> Result<CharacterId> {
  let archetype = CharacterFactory::new(Arc::new(pool.clone()))
    .create_npc(NPCTypeData {
      character: CharacterData::new(name.to_string(), CharacterType::NPC),
      base_stats: BaseStatsData::default(),
    })
    .await?;
  Ok(archetype.character.get().get_id()?)
}
