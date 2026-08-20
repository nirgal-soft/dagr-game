use std::{path::PathBuf, sync::Arc};

use crate::menu::terminal::TerminalSession;
use anyhow::{Context, Result};
use crossterm::event::KeyCode;
use dagr_lib::{
    Engine,
    campaign::{AcceptTagHook, CampaignArtifact, Error as CampaignError, TagContribution},
    characters::{CharacterFilter, CharacterKind, CharacterSummary},
    content::{
        FrontTargetKind, InstallPack, TagApplicability, TagCategoryFilter, TagCategorySchemaView,
        TagCategorySubject, TagDefinitionFilter, TagDefinitionSummaryView,
    },
    tagging::{
        AppliedTagSet, CandidateHook, CandidateHookFilter, SelectTags, TagApplicationFilter,
        TagCarrier, TagSet, TagSetFilter,
    },
};
mod render;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    NextItem,
    PreviousItem,
    NextPane,
    Draw,
    Apply,
    Replace,
    Accept,
    Reinstall,
    Refresh,
}

fn action_for_key(key: KeyCode) -> Option<Action> {
    match key {
        KeyCode::Tab => Some(Action::NextPane),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::NextItem),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::PreviousItem),
        KeyCode::Char('d') => Some(Action::Draw),
        KeyCode::Char('p') => Some(Action::Apply),
        KeyCode::Char('x') => Some(Action::Replace),
        KeyCode::Char('a') => Some(Action::Accept),
        KeyCode::Char('i') => Some(Action::Reinstall),
        KeyCode::Char('r') => Some(Action::Refresh),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    Categories,
    TagSets,
    Carriers,
    Hooks,
    Dangers,
}

impl Focus {
    const fn next(self) -> Self {
        match self {
            Self::Categories => Self::TagSets,
            Self::TagSets => Self::Carriers,
            Self::Carriers => Self::Hooks,
            Self::Hooks => Self::Dangers,
            Self::Dangers => Self::Categories,
        }
    }
}

pub async fn run(engine: Arc<Engine>, pack_path: PathBuf) -> Result<()> {
    let mut terminal = TerminalSession::open()?;
    let mut app = TagPlaytest::open(engine, pack_path).await?;
    loop {
        terminal.draw(|frame| app.draw(frame))?;
        let Some(key) = terminal.read_key()? else {
            continue;
        };
        if matches!(key, KeyCode::Char('q') | KeyCode::Esc) {
            return Ok(());
        }
        if let Some(action) = action_for_key(key) {
            if let Err(error) = app.dispatch(action).await {
                app.status = format!("Action failed: {error:#}");
            }
        }
    }
}

struct DangerTarget {
    artifact: CampaignArtifact,
    front_name: String,
    danger_name: String,
}

struct TagPlaytest {
    engine: Arc<Engine>,
    pack_path: PathBuf,
    focus: Focus,
    categories: Vec<TagCategorySchemaView>,
    category_index: usize,
    definitions: Vec<TagDefinitionSummaryView>,
    tag_sets: Vec<TagSet>,
    tag_set_index: usize,
    npcs: Vec<CharacterSummary>,
    npc_index: usize,
    applications: Vec<AppliedTagSet>,
    hooks: Vec<CandidateHook>,
    hook_index: usize,
    dangers: Vec<DangerTarget>,
    danger_index: usize,
    contributions: Vec<TagContribution>,
    status: String,
}

impl TagPlaytest {
    async fn open(engine: Arc<Engine>, pack_path: PathBuf) -> Result<Self> {
        let categories = engine
            .content()
            .tag_categories(TagCategoryFilter::default())
            .await
            .context("load tag categories")?;
        let category_index = categories
            .iter()
            .position(|category| category.subject == TagCategorySubject::Npc)
            .unwrap_or_default();
        let tag_sets = engine
            .tagging()
            .tag_sets(TagSetFilter::default())
            .await
            .context("load frozen Tag Sets")?;
        let npcs = engine
            .characters()
            .list(CharacterFilter {
                kind: Some(CharacterKind::Npc),
                ..CharacterFilter::default()
            })
            .await
            .context("load NPC carriers")?;
        let mut app = Self {
            engine,
            pack_path,
            focus: Focus::Categories,
            categories,
            category_index,
            definitions: Vec::new(),
            tag_set_index: tag_sets.len().saturating_sub(1),
            tag_sets,
            npcs,
            npc_index: 0,
            applications: Vec::new(),
            hooks: Vec::new(),
            hook_index: 0,
            dangers: Vec::new(),
            danger_index: 0,
            status: "Browse categories, draw a Tag Set, then apply it to a carrier".to_string(),
            contributions: Vec::new(),
        };
        app.refresh_selection().await?;
        Ok(app)
    }

    async fn dispatch(&mut self, action: Action) -> Result<()> {
        match action {
            Action::NextItem => self.move_selection(1).await?,
            Action::PreviousItem => self.move_selection(-1).await?,
            Action::NextPane => self.focus = self.focus.next(),
            Action::Draw => self.draw_tag_set().await?,
            Action::Apply => self.apply_selected(false).await?,
            Action::Replace => self.apply_selected(true).await?,
            Action::Accept => self.accept_hook().await?,
            Action::Reinstall => self.reinstall().await?,
            Action::Refresh => {
                let mut fresh = Self::open(self.engine.clone(), self.pack_path.clone()).await?;
                fresh.status = "Tag state refreshed from the Engine".to_string();
                *self = fresh;
            }
        }
        Ok(())
    }

    async fn move_selection(&mut self, delta: isize) -> Result<()> {
        match self.focus {
            Focus::Categories => {
                self.category_index =
                    wrapped_selection_index(self.category_index, self.categories.len(), delta);
                self.refresh_definitions().await?;
            }
            Focus::TagSets => {
                self.tag_set_index =
                    wrapped_selection_index(self.tag_set_index, self.tag_sets.len(), delta);
            }
            Focus::Carriers => {
                self.npc_index = wrapped_selection_index(self.npc_index, self.npcs.len(), delta);
                self.refresh_applications().await?;
                self.refresh_hooks().await?;
            }
            Focus::Hooks => {
                self.hook_index = wrapped_selection_index(self.hook_index, self.hooks.len(), delta);
            }
            Focus::Dangers => {
                self.danger_index =
                    wrapped_selection_index(self.danger_index, self.dangers.len(), delta);
            }
        }
        Ok(())
    }

    async fn draw_tag_set(&mut self) -> Result<()> {
        let category = self
            .categories
            .get(self.category_index)
            .context("no tag category is selected")?;
        let applicability = match category.subject {
            TagCategorySubject::Npc => TagApplicability::Npc,
            TagCategorySubject::Location => TagApplicability::Location(
                *category
                    .location_kinds
                    .first()
                    .context("selected Location category has no supported Location kinds")?,
            ),
        };
        let selection = self
            .engine
            .tagging()
            .snapshot()
            .await
            .select(SelectTags {
                category: category.content_key.clone(),
                applicability,
                count: None,
            })
            .context("draw Tag Set")?;
        let tag_set = self
            .engine
            .tagging()
            .create_tag_set(selection)
            .await
            .context("persist Tag Set")?;
        self.status = format!(
            "Drew frozen Tag Set {} from {}@{}",
            tag_set.id,
            tag_set.provenance.category.source_pack,
            tag_set.provenance.category.pack_version
        );
        self.refresh_tag_sets().await?;
        self.tag_set_index = self
            .tag_sets
            .iter()
            .position(|candidate| candidate.id == tag_set.id)
            .unwrap_or_default();
        Ok(())
    }

    async fn apply_selected(&mut self, replace: bool) -> Result<()> {
        let tag_set = self
            .tag_sets
            .get(self.tag_set_index)
            .context("no frozen Tag Set is selected")?;
        let npc = self
            .npcs
            .get(self.npc_index)
            .context("no NPC carrier exists")?;
        let tag_set_id = tag_set.id;
        let carrier = TagCarrier::Npc(npc.id);
        let npc_name = npc.name.clone();
        self.status = if replace {
            let outcome = self
                .engine
                .tagging()
                .apply_or_replace_tag_set(tag_set_id, carrier)
                .await
                .context("replace active Tag Set application")?;
            format!(
                "Applied Tag Set {} to {}; replaced applications {:?}",
                outcome.applied.application.tag_set_id, npc_name, outcome.replaced
            )
        } else {
            let application = self
                .engine
                .tagging()
                .apply_tag_set(tag_set_id, carrier)
                .await
                .context("apply Tag Set")?;
            format!("Applied Tag Set {} to {}", application.tag_set_id, npc_name)
        };
        self.refresh_applications().await?;
        self.refresh_hooks().await
    }

    async fn accept_hook(&mut self) -> Result<()> {
        let hook = self
            .hooks
            .get(self.hook_index)
            .context("no Candidate Hook is selected")?;
        let danger = self
            .dangers
            .get(self.danger_index)
            .context("no existing Danger is selected")?;
        let idempotency_key = format!(
            "tag-playtest:{}:{}",
            serde_json::to_string(&hook.key).expect("Candidate Hook keys are serializable"),
            match danger.artifact {
                CampaignArtifact::Danger(id) => id.to_string(),
                _ => unreachable!("tag playtest lists only Danger targets"),
            }
        );
        let contribution = self
            .engine
            .campaign()
            .accept_tag_hook(AcceptTagHook {
                hook: hook.key,
                idempotency_key,
                artifact: danger.artifact,
            })
            .await
            .context("accept Candidate Hook")?;
        self.status = format!(
            "Accepted into Danger {} with frozen Application {} provenance",
            danger.danger_name, contribution.hook.application.id
        );
        self.refresh_dangers().await
    }

    async fn reinstall(&mut self) -> Result<()> {
        let selected_key = self
            .categories
            .get(self.category_index)
            .map(|category| category.content_key.clone());
        self.engine
            .content()
            .install_pack(InstallPack {
                path: self.pack_path.clone(),
            })
            .await
            .context("reinstall authored content")?;
        self.categories = self
            .engine
            .content()
            .tag_categories(TagCategoryFilter::default())
            .await
            .context("reload tag categories")?;
        self.category_index = selected_key
            .and_then(|key| {
                self.categories
                    .iter()
                    .position(|category| category.content_key == key)
            })
            .unwrap_or(0);
        self.status = format!(
            "Reinstalled authored content from {}",
            self.pack_path.display()
        );
        self.refresh_selection().await
    }

    async fn refresh_selection(&mut self) -> Result<()> {
        self.refresh_definitions().await?;
        self.refresh_applications().await?;
        self.refresh_hooks().await?;
        self.refresh_dangers().await
    }

    async fn refresh_definitions(&mut self) -> Result<()> {
        let category = self
            .categories
            .get(self.category_index)
            .map(|category| category.content_key.clone());
        self.definitions = self
            .engine
            .content()
            .tag_definitions(TagDefinitionFilter {
                category,
                ..TagDefinitionFilter::default()
            })
            .await
            .context("load tag definitions")?;
        Ok(())
    }

    fn selected_carrier(&self) -> Option<TagCarrier> {
        self.npcs
            .get(self.npc_index)
            .map(|npc| TagCarrier::Npc(npc.id))
    }

    async fn refresh_tag_sets(&mut self) -> Result<()> {
        self.tag_sets = self
            .engine
            .tagging()
            .tag_sets(TagSetFilter::default())
            .await
            .context("load frozen Tag Sets")?;
        self.tag_set_index = self
            .tag_set_index
            .min(self.tag_sets.len().saturating_sub(1));
        Ok(())
    }

    async fn refresh_applications(&mut self) -> Result<()> {
        let carrier = self.selected_carrier();
        self.applications = self
            .engine
            .tagging()
            .applied_tag_sets(TagApplicationFilter {
                carrier,
                ..TagApplicationFilter::default()
            })
            .await
            .context("load applied Tag Sets")?;
        Ok(())
    }
    async fn refresh_hooks(&mut self) -> Result<()> {
        let carrier = self.selected_carrier();
        self.hooks = self
            .engine
            .tagging()
            .candidate_hooks(CandidateHookFilter {
                carrier,
                target_kind: Some(FrontTargetKind::Danger),
                ..CandidateHookFilter::default()
            })
            .await
            .context("load Candidate Hooks")?;
        self.hook_index = self.hook_index.min(self.hooks.len().saturating_sub(1));
        Ok(())
    }

    async fn refresh_dangers(&mut self) -> Result<()> {
        let mut dangers = Vec::new();
        for summary in self
            .engine
            .campaign()
            .active_fronts()
            .await
            .context("load active Fronts")?
        {
            let front = self
                .engine
                .campaign()
                .front(summary.front_id)
                .await
                .context("load active Front")?;
            for danger in front.dangers {
                dangers.push(DangerTarget {
                    artifact: CampaignArtifact::Danger(danger.danger_id),
                    front_name: front.name.clone(),
                    danger_name: danger.name,
                });
            }
        }
        self.dangers = dangers;
        self.danger_index = self.danger_index.min(self.dangers.len().saturating_sub(1));
        let mut contributions = Vec::new();
        for danger in &self.dangers {
            match self
                .engine
                .campaign()
                .tag_contribution(danger.artifact)
                .await
            {
                Ok(contribution) => contributions.push(contribution),
                Err(CampaignError::TagContributionNotFound { .. }) => {}
                Err(error) => return Err(error).context("load tag contribution"),
            }
        }
        self.contributions = contributions;
        Ok(())
    }
}

fn wrapped_selection_index(index: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        0
    } else if delta < 0 {
        (index + len - 1) % len
    } else {
        (index + 1) % len
    }
}
