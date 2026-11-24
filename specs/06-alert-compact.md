# Spec 06 — Alerta compacto com check e navegação

## Origem (pedido explícito)
> "ao exibir o alerta remova o conteudo acima do nome e coloque na esquerda um
> icone de check para marcar como finalizada e a direita um numero indicando as
> faltantes ao fazer check a tarefa sai com animação para esquerda e entra a
> proxima vindo dadireita efeito de fade nas bordas laterais"

## Requisito
O alerta mostra apenas: ícone de check à esquerda, nome da tarefa ao centro e
contador de pendentes à direita. Clicar no check conclui a tarefa atual: ela sai
animada para a esquerda e a próxima entra pela direita. Fade lateral nas bordas.

## Critérios de aceite
- AL-01: remover cabeçalho "PRÓXIMA"/contador antigo; linha única com check,
  marquee do nome e contador de pendentes.
- AL-02: check marca tarefa done, persiste, atualiza lista.
- AL-03: animação saída para esquerda + entrada pela direita da próxima tarefa.
- AL-04: mask/fade nas bordas laterais do marquee.
- AL-05: se não houver próxima, recolhe para idle.

## Rastreio
- Implementação: `src/ui.html`.
- Verificação: build release, inspeção visual.