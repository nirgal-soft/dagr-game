# Engine Tool Debug Console

Run the reference client with `cargo run` and choose **Debug engine tools** from
the main menu.

The console can:

- List the complete engine tool catalog and JSON schemas
- Inspect canonical GM, NPC, or Faction actor context
- Invoke any tool allowed for a bound actor scope using guided fields or raw JSON
- Recover from invalid menu choices, IDs, tool names, and arguments without exiting
- Display the structured result, refreshed context, and persisted engine events
- Create a demo NPC/Faction/Front scenario with IDs ready for testing

The guided mode prompts for each field in the tool schema and validates basic
value types. Raw JSON mode remains available for copy/paste and advanced testing;
invalid input can be corrected or cancelled with `:back`.

Example NPC relationship arguments:

```json
{"target_character_id":2,"change":15,"reason":"They shared the road ledger"}
```

Example NPC action attempt:

```json
{"description":"hides the map beneath the ledgers","intent":"conceal the map"}
```

Example Faction move attempt:

```json
{"description":"closes the eastern road","objective":"force the clans to negotiate"}
```

Example Front progression:

```json
{"danger_id":1,"reason":"The party ignored the threat for another week"}
```

The console calls the same serializable `ToolInvocation`/`ToolRouter` interface
used by model providers and future transport adapters. It does not bypass engine
validation or write directly to PostgreSQL.
