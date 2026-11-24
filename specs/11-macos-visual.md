# Spec 11 — Linguagem visual macOS recente

## Origem (pedido explícito)
> "metro cagou tudo deixe com a linguagem visual do osx mac mais recente"

## Requisito
Substituir a linguagem visual Metro por estilo macOS recente (Sequoia/Sonoma-like):
superfícies translúcidas/escuras suaves, cantos arredondados, tipografia do
sistema Apple/SF, controles pill/rounded, acento azul macOS.

## Critérios de aceite
- MV-01: `#notch` com cantos inferiores arredondados, fundo translúcido, blur,
  sombra suave; sem estética Metro chapada.
- MV-02: controles (botões, checks, inputs, selects, temas, badges) com raio
  arredondado e estados suaves.
- MV-03: tema padrão `macos` (`#0a84ff` / `#1e1e1e` / `#f5f5f7`).
- MV-04: migração única `macosMigrated` aplica tema macOS quando tema anterior
  era `metro` ou ausente/desconhecido; tarefas preservadas.
- MV-05: `shape_window` Win32 usa cantos inferiores arredondados.

## Rastreio
- Implementação: `src/ui.html`, `src/main.rs`.
- Verificação: `cargo build --release`, `/health`. Verificação visual manual
  pendente (sem screenshot automático).