# Spec 08 — Hover 1s antes de expandir + altura visível do idle

## Origem (pedido explícito)
> "e deve ficar com o hover no minimo 1seg sobre antes de expandir permita
> customizar altura visivel (que é difernete da altura da captura do gesto)"

## Requisito
- Hover na barra idle só expande após 1s de permanência.
- Altura visível do modo idle configurável (1–30 px). A área de captura do gesto
  permanece a janela/topo (não reduzida pela altura visível menor que a borda).

## Critérios de aceite
- HD-01: `pointerenter` inicia timer de 1000ms; se sair antes, cancela.
- HD-02: `idleVisibleHeight` em settings (1–30), persistido, aplicado na UI e no
  startup do Rust; propagado via IPC `idleHeight`.

## Rastreio
- Implementação: `src/ui.html`, `src/main.rs`.
- Verificação: build release.