# Spec 10 — Renomear para Metro Notch e linguagem visual Metro (WP8)

## Origem (pedido explícito)
> "renomeie publique no meu git esse projeto para metro notch e copie a
> linguagem visual do Metro windows phone 8 nele"

## Requisito
- Nome do projeto: **Metro Notch** (pacote/binário `metro-notch`).
- Publicação no GitHub do usuário (`vinicioslc`).
- UI com a linguagem visual Metro do Windows Phone 8: superfícies chapadas,
  cantos retos, tipografia Segoe UI Light, acento ciano `#1BA1E2`, sem
  gradientes, brilhos ou vidro.

## Critérios de aceite
- MN-01: `Cargo.toml` com `name = "metro-notch"`; binário `metro-notch.exe`.
- MN-02: título da janela/HTML "Metro Notch"; `serverInfo.name` = `metro-notch`.
- MN-03: estado em `%APPDATA%\metro-notch\state.json`, com migração única do
  arquivo antigo `%APPDATA%\notch-reminder\state.json` (tarefas preservadas).
- MN-04: autostart grava `MetroNotch` em HKCU Run e remove a entrada legada
  `NotchReminder`.
- MN-05: janela sem cantos arredondados (`SetWindowRgn` retangular).
- MN-06: CSS Metro: fundo chapado `var(--bg)`, borda/acento `#1BA1E2`, raio 0 em
  botões, checks, tiles, campos e contadores; sem `linear-gradient` decorativo,
  sem `box-shadow` de brilho, sem animação de varredura no idle.
- MN-07: tema padrão `metro` (`#1ba1e2` / `#000000` / `#ffffff`); paleta antiga
  migrada uma vez (flag `metroMigrated`), sem tocar nas tarefas.
- MN-08: repositório publicado em `github.com/vinicioslc/metro-notch`.

## Rastreio
- Implementação: `Cargo.toml`, `src/main.rs`, `src/http.rs`, `src/ui.html`.
- Docs: `README.md`, `REQUISITOS.md`, `CONNECTIVITY.md`, `specs/*`.
- Verificação: `cargo build --release`, `/health`, `/mcp` (`initialize`),
  `/api/tasks` preservando tarefas após a migração.
