# Spec 07 — Hover sem tarefas e altura compacta

## Origem (pedido explícito)
> "quando não possuir tarefas hover expande por completo diminua a altura no modo compact"

## Requisito
- Sem tarefas pendentes, hover na barra idle expande direto para a lista completa.
- Modo compacto (alerta) com altura reduzida.

## Critérios de aceite
- HE-01: `pointerenter` no idle com zero tarefas pendentes chama `setView('open')`.
- HE-02: altura do alerta reduzida de 58px para 40px (CSS, `targetSize`, `showAlert`).

## Rastreio
- Implementação: `src/ui.html`.
- Verificação: build release + inspeção visual.