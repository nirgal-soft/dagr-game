use std::{
  fs,
  sync::Arc,
  time::{SystemTime, UNIX_EPOCH},
};

use dagr_lib::{Engine, EngineConfig, NewWorld, campaign::CampaignArtifact};
use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::TestBackend};

use super::{TagPlaytest, action_for_key};

#[tokio::test]
async fn user_can_browse_draw_apply_and_inspect_tags_on_a_real_npc() {
  let root = std::env::temp_dir().join(format!(
    "dagr-tag-playtest-{}-{}",
    std::process::id(),
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  ));
  fs::create_dir_all(&root).unwrap();
  let core_content = crate::startup::core_content_path();
  let world_path = root.join("world.sqlite3");
  let engine = Arc::new(
    Engine::open(EngineConfig {
      world_path: world_path.clone(),
      core_content_path: core_content.clone(),
      new_world: Some(NewWorld { seed: 103 }),
    })
    .await
    .unwrap(),
  );
  let scenario = crate::debug_scenario::create(engine.clone()).await.unwrap();
  let mut app = TagPlaytest::open(engine.clone(), core_content.clone())
    .await
    .unwrap();

  press_key(&mut app, KeyCode::Down).await;
  press_key(&mut app, KeyCode::Down).await;
  assert!(screen(&app).contains("NPC"));

  press_key(&mut app, KeyCode::Char('d')).await;
  let rendered = screen(&app);
  assert!(rendered.contains("Applications (1)"));
  assert!(rendered.contains("seed"));
  assert!(rendered.contains("core@"));
  assert!(rendered.contains("Candidate Hooks ("));

  press_key(&mut app, KeyCode::Char('a')).await;
  let contribution = engine
    .campaign()
    .tag_contribution(CampaignArtifact::Danger(scenario.danger_id))
    .await
    .unwrap();
  assert_eq!(
    contribution.artifact,
    CampaignArtifact::Danger(scenario.danger_id)
  );
  assert!(screen(&app).contains("Accepted into Danger"));
  drop(app);
  drop(engine);
  let replacement = root.join("replacement");
  copy_pack(&core_content, &replacement);
  fs::write(
    replacement.join("manifest.toml"),
    fs::read_to_string(replacement.join("manifest.toml"))
      .unwrap()
      .replace("0.1.0-dev", "0.2.0-playtest"),
  )
  .unwrap();
  fs::write(
    replacement.join("tags.toml"),
    fs::read_to_string(replacement.join("tags.toml"))
      .unwrap()
      .replace("name = \"", "name = \"Current "),
  )
  .unwrap();
  let reopened = Arc::new(
    Engine::open(EngineConfig {
      world_path,
      core_content_path: core_content.clone(),
      new_world: None,
    })
    .await
    .unwrap(),
  );
  let mut reopened_app = TagPlaytest::open(reopened, replacement).await.unwrap();
  let reopened_screen = screen(&reopened_app);
  assert!(reopened_screen.contains("Applications (1)"));
  assert!(reopened_screen.contains("Contribution to The Brass"));
  assert!(reopened_screen.contains("core@0.1.0-dev"));

  press_key(&mut reopened_app, KeyCode::Char('i')).await;
  assert!(screen(&reopened_app).contains("Current "));
  press_key(&mut reopened_app, KeyCode::Down).await;
  press_key(&mut reopened_app, KeyCode::Down).await;
  press_key(&mut reopened_app, KeyCode::Char('d')).await;
  let replaced_screen = screen(&reopened_app);
  assert!(replaced_screen.contains("Applications (2)"));
  assert!(replaced_screen.contains("core@0.1.0-dev"));
  assert!(replaced_screen.contains("core@0.2.0-playtest"));

  fs::remove_dir_all(root).unwrap();
}

async fn press_key(app: &mut TagPlaytest, key: KeyCode) {
  let action = action_for_key(key).expect("test key maps to a Tag Playtest action");
  app.dispatch(action).await.unwrap();
}

fn copy_pack(source: &std::path::Path, destination: &std::path::Path) {
  fs::create_dir_all(destination).unwrap();
  for entry in fs::read_dir(source).unwrap() {
    let entry = entry.unwrap();
    fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
  }
}

fn screen(app: &TagPlaytest) -> String {
  let backend = TestBackend::new(120, 40);
  let mut terminal = Terminal::new(backend).unwrap();
  terminal.draw(|frame| app.draw(frame)).unwrap();
  terminal
    .backend()
    .buffer()
    .content
    .iter()
    .map(|cell| cell.symbol())
    .collect()
}
