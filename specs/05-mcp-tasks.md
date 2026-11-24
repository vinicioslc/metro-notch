# Spec 05 — MCP HTTP publicado + tarefas organizadas internamente

## Origem (pedido explícito)
> "ele ja expoe o mcp ? publique o mcp http nele expondo listagem de tarefas e
> etc organize como tasks as tarefas dele internamente"

## Requisito
Confirmar/publicar o MCP HTTP e organizar as tarefas internas como um modelo de
tarefa único, exposto por MCP e REST.

## Critérios de aceite
- MC-01: `POST http://127.0.0.1:7878/mcp` responde `initialize`, `tools/list`,
  `tools/call` (JSON-RPC 2.0); `NOTCH_BIND` permite publicar em outra interface.
- MC-02: tools de tarefas: `list_tasks` (filter), `get_task`, `add_task`,
  `update_task`, `complete_task`, `remove_task`, `reorder_tasks`, `stats`.
- MC-03: modelo interno `{id, name, description, done, createdAt, updatedAt}`.
- MC-04: REST `GET /api/tasks?filter=all|pending|done`, `GET/PATCH/DELETE
  /api/tasks/{id}`.
- MC-05: tarefas antigas migradas (`text` → `name`, timestamps derivados do id).

## Rastreio
- Implementação: `src/http.rs`, `src/main.rs`, `src/ui.html`.
- Verificação: `tools/list`, `list_tasks`, `stats`, `GET /api/tasks?filter=pending`.