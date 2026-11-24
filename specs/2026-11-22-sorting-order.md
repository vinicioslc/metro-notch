# 2026-11-22-sorting-order.md

**Contexto:**  
Metro Notch app, UI settings panel, users want to control sorting of task list.

**Requisito:**  
Permitir ao usuário escolher a ordenação das tarefas nas configurações: “último adicionado primeiro”, “mais antigo primeiro” ou “mais recente primeiro”. A escolha deve ser persistida e refletida no display da lista e nos alertas.

**Critérios de aceitação:**  
1. Na página de configurações há um `<select>` com três opções.  
2. Selecionar uma opção reordena a lista de tarefas imediatamente.  
3. O modo de ordenação escolhido é salvo em `state.json` e carregado ao iniciar.  
4. A lista de tarefas continua sendo atualizada quando novas tarefas são adicionadas ou removidas.  
5. O alerta de próxima tarefa segue a ordem configurada (ex.: próximo = primeiro item na lista).  
5. Nenhum comportamento existente é alterado.

**Rastreamento:**  
- Código → Implementação: `src/ui.html` (UI, `<select>`, `sortSelect` listener, `sortTasks` helper).  
- Implementação → Verificação: testes manuais de UI, captura de tela da lista reordenada, verificação de `state.json` contendo `sort` key.

**Notas:**  
- IDs das tarefas (`id`) são numéricos baseados em `Date.now()`; para “mais antigo” e “mais recente” usamos comparação de `id`.  
- Se houver conflito de IDs (duas tarefas criadas ao mesmo milissegundo), a ordem de inserção (`unshift`/`push`) quebra o tie.