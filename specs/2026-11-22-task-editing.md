# 2026-11-22-task-editing.md

**Contexto:**  
Metro Notch app, lista de prioridades. Atualmente o texto da tarefa é apenas exibido; não pode ser editado após criação.

**Requisito:**  
Ao clicar no texto de uma tarefa na lista, permitir editar nome (e descrição) in-place. Confirmar com Enter/blur, cancelar com Esc. Persistir alterações.

**Critérios de aceitação:**  
1. Clicar no texto de uma tarefa abre um campo de edição inline.  
2. Enter (ou blur) salva o novo nome/descrição.  
3. Esc cancela e restaura o valor anterior.  
4. Alterações são persistidas em `state.json`.  
5. UI permanece responsiva e sem quebra de layout.

**Rastreamento:**  
- Código → Implementação: `src/ui.html` (renderList, editTask, event listeners).  
- Implementação → Verificação: testes manuais de UI, verificação de persistência.