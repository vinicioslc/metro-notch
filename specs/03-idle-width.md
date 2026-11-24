# Spec 03 — Largura do modo idle igual ao compacto

## Origem (pedido explícito)
> "faça a largura do modo hidden ser a mesma do modo compact"

## Requisito
A largura da janela no estado idle (hidden) deve ser igual à largura do modo
compacto (alert), eliminando deslocamento horizontal na expansão.

## Critérios de aceite
- IW-01: `IDLE_W` = largura do modo alert (560 px lógicos) em UI e Rust.

## Rastreio
- Implementação: `src/ui.html` (CSS width, targetSize, initial size), `src/main.rs`
  (`IDLE_W`).
- Verificação: build release + medição da janela.