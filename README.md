# Metro Notch

Barra de tarefas flutuante no topo da tela, com visual Metro (Windows Phone 8),
alerta periódico das pendências, MCP e REST locais.

- Repositório: `github.com/vinicioslc/metro-notch`
- Estado: `%APPDATA%\metro-notch\state.json`
- HTTP/MCP: `http://127.0.0.1:7878` (`NOTCH_PORT`, `NOTCH_BIND`)

## Build

```powershell
cargo build --release
.\target\release\metro-notch.exe
```

## MCP

`POST http://127.0.0.1:7878/mcp` (JSON-RPC 2.0). Tools: `list_tasks`,
`get_task`, `add_task`, `update_task`, `complete_task`, `remove_task`,
`reorder_tasks`, `stats`, `get_settings`, `set_settings`, `get_state`,
`set_state`, `trigger_alert`, `read_screen`, `chat`, `quit`.

REST: `/health`, `/api/tasks`, `/api/tasks/{id}`, `/api/settings`,
`/api/screen`, `/api/chat`, `/api/llm`.

Detalhes em [CONNECTIVITY.md](CONNECTIVITY.md) e [REQUISITOS.md](REQUISITOS.md).
Specs em [specs/](specs/).
