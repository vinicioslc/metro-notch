# Spec 01 — Conectividade (MCP + IPC)

## Origem (pedido explícito)
> "precisamos que esse projeto consiga ser conectado em qualquer outra aplicação
> com um protocolo conhecido, como mcp para iAs e inter process para integrações
> mais completas como um plugin para o local-ontology"

## Requisito
O app deve expor seus dados (tarefas + configurações) por protocolos conhecidos
para que outras aplicações e IAs se integrem.

## Critérios de aceite
- CN-01: servidor HTTP embutido em `127.0.0.1:7878` (override `NOTCH_PORT`),
  sem dependências novas (std::net).
- CN-02: endpoint MCP `POST /mcp` (JSON-RPC 2.0) com tools de tarefas/config/estado.
- CN-03: endpoints REST `/api/*` para integração inter-processo.
- CN-04: mutações externas propagadas para a UI via `window.__load(...)`.
- CN-05: documentação em `CONNECTIVITY.md` e adapters em `integrations/`.

## Rastreio
- Implementação: `src/http.rs`, `src/main.rs` (shared state + http::start).
- Verificação: build release + `/health` + `tools/list` + `add_task`/`list_tasks`.