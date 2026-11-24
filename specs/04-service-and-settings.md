# Spec 04 — Serviço (autostart) + mais personalizações (tempos)

## Origem (pedido explícito)
> "instale como serviço adicione mais personalizacoes tempos e etc"

## Requisito
O app deve poder iniciar com o Windows (autostart via HKCU Run — um serviço
Windows real não pode exibir UI, por isso usa autostart no logon) e oferecer
mais personalizações de tempo: duração do alerta, horário de silêncio (quiet
hours) e toggle de autostart.

## Critérios de aceite
- SV-01: toggle "Iniciar com o Windows" escreve/remove `HKCU\...\Run\MetroNotch`.
- SV-02: `alertSeconds` (1–60, padrão 8) controla a duração do alerta e a
  animação de progresso.
- SV-03: `quietStart`/`quietEnd` (HH:MM) suprimem alertas automáticos dentro do
  intervalo (cruza meia-noite).
- SV-04: novos campos expostos via `/api/settings` e MCP `set_settings`.

## Rastreio
- Implementação: `src/main.rs` (set_autostart), `src/ui.html`, `src/http.rs`.
- Verificação: build release + toggle autostart (HKCU) + alterar duração/silêncio.