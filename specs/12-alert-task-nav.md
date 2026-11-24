# Spec 10 - Chevrons para navegar entre tarefas no alerta

## Origem (pedido explícito)
> "coloque uma seta esquerda (oculta quando no primeiro) e direita (oculta
> quando na ultima) discreta chevron dos lados do nome da tarefa atual
> permitindo ciclar atraves das anteriores e ver"

## Requisitos
- NV-01: no alerta (barra de 40 px), uma chevron discreta à esquerda e outra à
  direita do nome da tarefa exibida.
- NV-02: a esquerda fica oculta quando a tarefa exibida é a primeira da lista
  de pendentes; a direita fica oculta quando é a última.
- NV-03: clicar numa chevron mostra a tarefa vizinha (anterior/próxima) na
  mesma barra, com a mesma animação de troca do alerta, sem abrir o painel.
- NV-04: a navegação só percorre tarefas pendentes (não concluídas), na mesma
  ordem em que os alertas as escolhem; ao chegar num limite, a chevron daquele
  lado desaparece.
- NV-05: com uma única tarefa pendente, nenhuma chevron aparece.
- NV-06: o clique na chevron não dispara o clique do alerta que abre o painel.

## Critérios de aceite
- AC-01: com 3 pendentes e a primeira exibida, só a chevron direita aparece;
  clicar nela mostra a segunda; na terceira, só a esquerda aparece.
- AC-02: concluir a tarefa exibida atualiza as chevrons conforme a nova lista.
- AC-03: as chevrons não alteram a altura da barra (continua 40 px) nem o
  comportamento de hover/clique existente.

## Rastreio
- Implementação: `src/ui.html` — markup `.alert-row`, CSS `.alert-nav`,
  funções `alertIndex`/`updateAlertNav`/`stepAlert`, chamada em `showAlert`.
- Verificação: `node --check` no script extraído + teste manual na barra.
