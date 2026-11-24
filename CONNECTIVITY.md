# Conectividade — Metro Notch

O Metro Notch expõe seus dados (tarefas + configurações) por protocolos
conhecidos para que outras aplicações e IAs possam se integrar.

## Endpoint

- Host: `127.0.0.1` (override via env `NOTCH_BIND`, ex.: `0.0.0.0` para expor na rede)
- Porta padrão: `7878` (override via env `NOTCH_PORT`)
- Protocolo: HTTP/1.1, JSON. CORS permissivo.

> Expor com `NOTCH_BIND=0.0.0.0` deixa o MCP/REST acessível na rede local sem
> autenticação. Use apenas em rede confiável.

## MCP (para IAs)

`POST http://127.0.0.1:7878/mcp` — JSON-RPC 2.0 (mesmo formato do
local-ontology). Compatível com Claude Desktop, Cursor, etc.

Tools:
- `get_state`, `set_state`
- `list_tasks` (filter `all|pending|done`), `get_task`
- `add_task`, `update_task`, `complete_task`, `remove_task`, `reorder_tasks`
- `stats`
- `get_settings`, `set_settings`
- `read_screen` (texto da janela ativa via UI Automation), `chat` (LLM local)
- `trigger_alert`, `quit`

Modelo de tarefa interno:
```json
{"id": 179..., "name": "...", "description": "...", "done": false,
 "createdAt": 179..., "updatedAt": 179...}
```

Exemplo de config do cliente (`mcpServers`):

```json
{
  "mcpServers": {
    "metro-notch": {
      "url": "http://127.0.0.1:7878/mcp"
    }
  }
}
```

## IPC / REST (integrações mais completas)

- `GET  /health`
- `GET  /api/state` / `POST /api/state`
- `GET  /api/tasks?filter=all|pending|done` / `POST /api/tasks`
- `GET  /api/tasks/{id}` / `PATCH /api/tasks/{id}` / `DELETE /api/tasks/{id}`
- `GET  /api/settings` / `POST /api/settings`
- `GET  /api/screen?chars=4000` — texto da janela em primeiro plano (UI Automation)
- `GET  /api/llm` — runtime/modelo do chat e se está disponível
- `POST /api/chat` — `{"messages":[…],"screen":true}` → `{"reply":"…"}`
- `POST /api/quit`

## Chat local (LLM)

O chat usa o runtime já instalado na máquina (Ollama em `127.0.0.1:11434`),
com o texto da tela como contexto. Configuração por env:

| Variável | Padrão | Efeito |
|---|---|---|
| `METRO_LLM_URL` | `http://127.0.0.1:11434` | runtime HTTP compatível |
| `METRO_LLM_MODEL` | `lfm3:1.7b` | modelo instalado |
| `METRO_LLM_CTX` | `4096` | janela de contexto |
| `METRO_LLM_GPU_LAYERS` | `0` | camadas na GPU; `0` = CPU, 0 MB de VRAM |

Para um modelo mais rápido: `ollama pull qwen3:0.6b` e
`METRO_LLM_MODEL=qwen3:0.6b`. Acima de ~10 camadas na GPU o contexto CUDA já
passa do teto de 500 MB, então o padrão é CPU.

## Push para a UI

Mutações feitas via HTTP/MCP são propagadas para a janela do app
automaticamente (eval `window.__load(...)`), então a interface reflete o estado
externo sem reiniciar.

## Plugin para o local-ontology

Ver `integrations/local_ontology_plugin.py` — adapter drop-in que lê o estado do
Metro Notch e ingere como documento no workspace do local-ontology.