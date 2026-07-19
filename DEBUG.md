# Engine Tool Debug Console

Run the reference client with `cargo run` and choose **Debug engine tools** from
the main menu.

The console can:

- List the complete engine tool catalog and JSON schemas
- Browse GM, NPC, and Faction actors by name instead of memorizing IDs
- Inspect canonical actor context, including creative GM state
- Invoke any tool allowed for a bound actor scope using guided fields or raw JSON
- Recover from invalid menu choices, IDs, tool names, and arguments without exiting
- Display the structured result, changed context sections, and persisted events
- Create a demo NPC/Faction/Front scenario with IDs ready for testing

The guided mode prompts for each field in the tool schema and validates basic
value types. Raw JSON mode remains available for copy/paste and advanced testing;
invalid input can be corrected or cancelled with `:back`.

Creative GM tools can frame scenes, apply batches of facts/abilities/decisions,
introduce new Front Dangers, and resolve attempts or player decisions. Nested
effect arrays are easiest to paste through raw JSON mode.

Example creative GM effect batch:

```json
{"summary":"Make the gate matter","effects":[{"type":"establish_fact","subject":"The eastern gate","assertion":"It opens only for someone carrying funeral ash","visibility":"public"},{"type":"offer_decision","prompt":"Who carries the ash?","options":["Amleth","Beatrice"],"stakes":"The gate marks whoever enters first"}]}
```

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
