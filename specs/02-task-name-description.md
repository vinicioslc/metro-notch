# Spec 02 — Tarefa com nome e descrição

## Origem (pedido explícito)
> "permita uma tarefa ter nome e ter descrição descrição tem tamanho de até
> 250000 caracteres nome tem 200 caracteres"

## Requisito
Cada tarefa passa a ter `name` (≤ 200 chars) e `description` (≤ 250 000 chars).

## Critérios de aceite
- TD-01: modelo de tarefa `{id, name, description, done}`.
- TD-02: validação/truncagem em 200 (name) e 250 000 (description) em UI e servidor.
- TD-03: migração de tarefas antigas (`text` → `name`).
- TD-04: HTTP/MCP aceitam `name`/`description` (e `text` como alias de `name`).

## Rastreio
- Implementação: `src/ui.html`, `src/http.rs`, `src/main.rs` (defaults).
- Verificação: build release + `POST /api/tasks` com name/description + `list_tasks`.