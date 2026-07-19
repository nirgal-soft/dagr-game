# Engine Workbench

Run `cargo run`, choose **Debug engine tools**, and DAGR opens a full-screen
terminal workbench over the same `ToolRouter` used by model providers.

## Navigation

- `Tab` / `Shift-Tab`: move between Actors, Capabilities, and Details
- `↑` / `↓` or `j` / `k`: navigate the focused pane
- `Enter`: select an actor, open a tool, or toggle the detail view
- `/`: live filter and autocomplete the focused actor/tool list
- `e`: toggle canonical context and the event timeline
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
