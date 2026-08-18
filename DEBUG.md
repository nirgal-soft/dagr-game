# Debug and Playtest Controls

## World Hex editor

Press `:` and enter `debug on` to enable session-local world editing. In World
view, press `x` to open the Hex placement overlay. Move the world cursor with
`hjkl/yubn`, select profile fields with `↑`/`↓`, cycle values with `←`/`→`, and
press `x` to atomically create the configured Hex at an empty coordinate. `Esc`
closes the editor. Existing Hexes are never overwritten.

The first profile editor controls terrain, vegetation, water, POI, climate,
danger, resource richness, and a currently empty Region selection. Commands are
limited to `debug on`, `debug off`, `debug status`, and `help`; the console does
not execute SQL or mutate ECS directly.

## Combat and local inspection

Choose **Combat Arena** from the main menu to enter a persistent, fully visible
31×21 stone-circle test area. It has a separate location identity, so arena
enemies do not leak into the generated Wilderness and can later retain wounds
across restarts.

In **Play game** or the arena, press `M` to open the monster picker. Type any
subsequence of a monster name or stable key, use `↑`/`↓` to select, and press
`Enter` to instantiate it on a nearby walkable tile. `Esc` cancels. The picker
reads the loaded bestiary, so new content-pack monsters appear without client
code changes.

In the arena, moving into a monster exchanges one Whitehack-style attack and one
retaliation. The arena fighter is a durable level-5 Strong test fixture. HP,
wounds, defeat, and tile positions persist, while dice outcomes accumulate in
the non-modal **Combat rolls** panel. Press `R` to clear positioned arena
monsters and restore the arena fighter to full HP. Arena monsters notice the
fighter within eight tiles, advance one tile after each player action, and attack
when adjacent. Press `.` to wait and exercise enemy turns directly.

Press `x` in a local area to enter free Look mode. Movement keys reposition the
inspection cursor without consuming a turn, the right-hand panel describes the
visible creature, fixtures, POI, terrain feature, and ground under the cursor,
and `x` or `Esc` exits. Decorative fixtures such as bedrolls can carry client
names and descriptions without becoming persistent backend entities. Press `o` or
`O` to auto-explore other local areas; this also dismisses an open discovery or
enemy popup so exploration starts immediately.

## Engine Workbench

Run `cargo run`, choose **Debug engine tools**, and DAGR opens a full-screen
terminal workbench over the same Agency tool catalog and invocation seam used
by model providers.

## Navigation

- `Tab` / `Shift-Tab`: move between Actors, Capabilities, and Details
- `↑` / `↓` or `j` / `k`: navigate the focused pane
- `Enter`: select an actor, open a tool, or toggle the detail view
- `/`: live filter and autocomplete the focused actor/tool list
- `e`: toggle canonical context and the event timeline
- `L`: open the AI-powered Scene Playground; `Esc` returns to the workbench
- `PageUp` / `PageDown`: scroll details
- `r`: refresh canonical state
- `d`: create a ready-to-use demo NPC/Faction/Front scenario
- `?`: contextual help
- `q`: return to the main menu

## Tool forms

Opening a capability produces a schema-generated form. Existing field values are
retained after validation failures.

- Type and Backspace edit the selected field
- `Tab` / `↑` / `↓` move between fields
- `←` / `→` cycle enum choices
- `Ctrl-U` clears a field
- `F2` loads an editable example for the selected tool
- `F5` validates and commits
- `Esc` cancels without changing engine state

Arrays, objects, and creative effect batches accept JSON in their individual
field editors. The complete value remains visible and wrapped while editing, so
pasted effects do not disappear into a one-line prompt.

Successful calls refresh canonical context and appear immediately in the event
timeline. Failures remain in the form beside the original input for correction.
The workbench never bypasses Engine validation or writes directly to SQLite.

## Scene Playground

Press `L` from the workbench to test the smallest fun unit of play: one immediate
situation, one pressure, and one invitation to act. Choose the GM, an NPC, or a
Faction to animate the moment. `F2` cycles editable scene starters, `F3` begins a
fresh vignette, and ordinary messages continue play for a few short turns.

The default view presents `STORY` and concise `WORLD` changes rather than debug
payloads. Press `V` when technical event detail is useful. Actor search, scene
scrolling, retained short-term history, missing-key errors, and clean TTY
transitions are handled in place.

The agent runtime defaults to local Ollama with `qwen3:4b` for both model tiers.
Use the namespaced `DAGR_AGENT_*` policy in `.env` to select a remote Ollama URL,
Anthropic, profile models, output caps, or exact semantic-role tier overrides.
Anthropic additionally requires `ANTHROPIC_API_KEY`. The separate
`DAGR_STRUCTURED_*` settings control schema-constrained generation. Construction
validates configuration but does not probe either provider. Story prose is
presentation only. Green `WORLD` cards represent validated canonical changes.
