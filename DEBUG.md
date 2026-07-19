# Debug and Playtest Controls

In **Play game**, press `M` while inside a Wilderness or dungeon to create one
persistent `core:goblin` test enemy on a nearby walkable tile. The enemy is
rendered only in line-of-sight, blocks movement, and is rehydrated at the same
location and tile after restart. Repeating `M` in the same area does not create
a duplicate. This is an enemy-instantiation test; combat and AI are intentionally
not part of this slice.

## Engine Workbench

Run `cargo run`, choose **Debug engine tools**, and DAGR opens a full-screen
terminal workbench over the same `ToolRouter` used by model providers.

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
The workbench never bypasses engine validation or writes directly to PostgreSQL.

## Scene Playground

Press `L` from the workbench to test the smallest fun unit of play: one immediate
situation, one pressure, and one invitation to act. Choose the GM, an NPC, or a
Faction to animate the moment. `F2` cycles editable scene starters, `F3` begins a
fresh vignette, and ordinary messages continue play for a few short turns.

The default view presents `STORY` and concise `WORLD` changes rather than debug
payloads. Press `V` when technical event detail is useful. Actor search, scene
scrolling, retained short-term history, missing-key errors, and clean TTY
transitions are handled in place.

Set `ANTHROPIC_API_KEY` in `.env` to play. The default model is the inexpensive
`claude-haiku-4-5-20251001`; override it with `DAGR_LLM_MODEL`. Story prose is
presentation only. Green `WORLD` cards represent validated canonical changes.
