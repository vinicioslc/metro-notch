# Integrações

Arquivos auxiliares para conectar o Metro Notch a outras aplicações.

| Arquivo | Uso |
|---------|-----|
| `claude_desktop_config.json` | Exemplo de `mcpServers` para Claude Desktop. |
| `local_ontology_plugin.py` | Adapter Python que lê o estado do Metro Notch e ingere no local-ontology (via `/mcp` ou REST). |
| `sync_to_local_ontology.py` | Script de sincronização única: tarefas pendentes do Metro Notch → documento do local-ontology. |

O app deve estar rodando e expõe por padrão `http://127.0.0.1:7878`.