# Requisitos — Metro Notch

Aplicativo de lembrete para Windows que fica pendurado na borda superior da tela,
mostra os objetivos do dia em intervalos configuráveis e abre uma lista de
prioridades ao passar o mouse.

Idioma de implementação: **Rust**. Documento de requisitos consolidado; cada item
tem um código para rastreio (código → implementação → verificação).

---

## 1. Requisitos funcionais

| Código | Requisito | Origem |
|---|---|---|
| RF-01 | Barra "notch" ancorada na borda superior da tela, centralizada, sem moldura de janela nativa. | pedido inicial |
| RF-02 | Adicionar objetivos do dia (campo de texto + botão). | pedido inicial |
| RF-03 | Alertar a cada X minutos ou horas (configurável), descendo e expandindo para mostrar a tarefa. | pedido inicial |
| RF-04 | Rotacionar entre as tarefas pendentes a cada alerta (não repetir a mesma seguida). | implementação |
| RF-05 | Marcar / desmarcar tarefa como concluída, com check animado. | pedido inicial |
| RF-06 | Reordenar a lista arrastando as linhas (drag & drop). | pedido inicial |
| RF-07 | Remover tarefa individualmente. | implementação |
| RF-08 | Ao passar o mouse (hover) na barra parada, revelar a barra compacta com a tarefa atual; **clicar** abre a lista completa de prioridades. | pedido inicial + steer |
| RF-09 | Botão de engrenagem (cog) no canto do painel abre as configurações. | pedido inicial |
| RF-10 | Configurações: intervalo numérico + unidade (min / h). | pedido inicial |
| RF-11 | Configurações: paleta personalizada — cor da borda, do fundo e do texto. | pedido inicial |
| RF-12 | Temas prontos: **Nord, Monokai, Cobalt, GitHub Dark, GitHub Light**. | pedido inicial |
| RF-13 | Restaurar configurações padrão (mantendo as tarefas). | implementação |
| RF-14 | Encerrar o aplicativo pela própria interface. | implementação |
| RF-15 | Ficar sobre / expandir sobre qualquer aplicativo em execução. | pedido inicial |
| RF-19 | Ordenar tarefas por data (último adicionado, mais antigo, mais recente) | pedido inicial |
| RF-16 | Ocultar automaticamente quando o cursor sai: **espera de 2 segundos** antes de recolher. | steer |
| RF-17 | Fundo com **blur de vidro** (frosted glass) atrás do painel. | steer |
| RF-18 | Documentar os requisitos (este arquivo). | steer |

## 2. Requisitos de UI/UX e animação

| Código | Requisito | Origem |
|---|---|---|
| UI-01 | Estado idle: apenas um leve traço com bordas curvas, com sensação de sair da tela para cima. | pedido inicial |
| UI-02 | Idle muito fino: **no máximo ~5 px** de altura (≈ 2 mm). | pedido inicial |
| UI-03 | Alerta: desce expandindo compacto, o suficiente para o texto, com letreiro animado e **fades horizontais**. | pedido inicial + ajuste |
| UI-04 | Aparência bonita e moderna, com micro animações e interações em todo o app (hover, check, arrasto, abertura/fechamento, cog girando, pulso, respiração do traço, barra de progresso que drena). | pedido inicial |
| UI-05 | Bordas de baixo **bem arredondadas**; topo reto, encostado na borda da tela. | steer |
| UI-06 | Temas aplicam-se à paleta inteira (borda, fundo, texto) e também ao tom do vidro. | implementação |
| UI-07 | Barra compacta e lista nunca ficam maiores que o conteúdo: nada de área de vidro vazia antes do conteúdo expandir. | bug relatado |
| UI-08 | Painel de vidro legível sobre qualquer fundo: fundo a 88 %→72 % de opacidade em vez de dissolver para transparente. | steer |

### Desvios conscientes

- **UI-03 × altura real:** o alerta compacto tem **40 px** (check + nome + contador + barra de progresso). Ajuste aplicado em `specs/07`.
- **RF-08:** a lista completa abre por **clique** na barra compacta, não
  diretamente no hover — a barra idle de 5 px é pequena demais para acertar e a
  lista é grande demais para abrir só passando o mouse.
- **RF-19:** o tema padrão é aplicado no tint do vidro; temas claros deixam o
  painel claro por causa do fundo da própria página.

## 3. Requisitos técnicos

| Código | Requisito |
|---|---|
| RT-01 | Escrito em Rust, binário único, **sem Node/npm/Tauri**: `tao` (janela) + `wry` (WebView2), UI em HTML/CSS/JS embutida com `include_str!`. |
| RT-02 | Janela transparente, sem decoração, fora da barra de tarefas, sempre no topo. |
| RT-03 | Estilo de janela `WS_POPUP` + `DwmSetWindowAttribute`: sem moldura, sem canto arredondado do sistema, sem borda; é o que permite a janela ter 5 px de altura (o estilo padrão do Windows força um mínimo de ~38 px). |
| RT-04 | Alfa por pixel via `DwmEnableBlurBehindWindow` (região vazia) + `SetWindowCompositionAttribute` (acrílico) carregado por `GetProcAddress`, porque a API não existe na import library do SDK. |
| RT-05 | Comunicação Rust ↔ página por IPC JSON: `load`, `save`, `size`, `quit`. |
| RT-06 | Tamanho da janela acompanha a animação do painel quadro a quadro, interpolado pelo progresso da própria transição CSS (leitura de layout atrasa e deixa faixa de vidro). |
| RT-07 | Persistência em `%APPDATA%\metro-notch\state.json`. |
| RT-08 | Topmost reafirmado periodicamente para continuar acima de apps que se elevam. |
| RT-09 | Build de release com `windows_subsystem = "windows"` (sem console). |

## 4. Bugs encontrados e corrigidos

| Código | Sintoma | Causa raiz |
|---|---|---|
| BF-01 | Janela nunca ficava com 5 px (mínimo ~38 px). | Estilo `WS_DLGFRAME` do tao faz o Windows impor altura mínima de rastreio. Corrigido trocando para `WS_POPUP`. |
| BF-02 | Interface completamente invisível / sem persistência / sem reação ao mouse. | A página declarava `function ipc()`, mas o wry já injeta `window.ipc` não configurável: `SyntaxError: Identifier 'ipc' has already been declared` derrubava **todo** o script. Renomeado para `send()`. |
| BF-03 | Área com blur aparecia antes do conteúdo expandir. | A janela era dimensionada por `getBoundingClientRect()`, que fica atrás do valor animado (e travar a animação deixava a janela presa no meio). Agora o tamanho vem do progresso da transição. |
| BF-04 | Faixa clara no topo do painel, parecendo blur sobrando. | Não era geometria: layout conferido na página (`notch: [0,0,560,258]` dentro de viewport `560x259`). Era o fundo do app atrás (barra de título clara) atravessando o painel a 70 % de opacidade. Resolvido subindo a opacidade do fundo do painel. |

## 5. Verificação

| Item | Como foi verificado |
|---|---|
| UI-02 | Retângulo real da janela medido por `GetWindowRect` com processo DPI-aware: `210x6` físicos = `168x5` lógicos (escala 125 %). |
| UI-03 / RF-03 | Alerta medido em `700x73` físicos = `560x58` lógicos; letreiro e barra de progresso visíveis em captura. |
| RF-08 / RF-09 | Hover e clique dirigidos por `SetCursorPos`/`mouse_event`: alerta `700x73` → lista `700x273` (3 tarefas) / `700x323` (4 tarefas) → configurações `700x468`. |
| UI-07 | `notch` medido dentro da página = `[0,0,560,258]` e janela `560x259` lógicos: painel e janela sempre do mesmo tamanho. |
| RF-16 | Cursor movido para fora: volta a `210x6` após ~2 s. |
| RT-05 / RT-07 | Mensagem IPC de teste gravou em arquivo; `state.json` inspecionado. |
| BF-02 | `window.onerror` injetado em fase de depuração capturou o `SyntaxError`. |

## 6. Build e execução

```powershell
cd C:\git\metro-notch
cargo build --release          # ~10 s incremental
.\target\release\metro-notch.exe
```

- Não abre console (subsistema Windows); para encerrar use "Sair" nas configurações
  ou `Get-Process metro-notch | Stop-Process -Force`.
- Não se registra em autostart: rodar manualmente ou criar atalho na pasta Inicializar.
- Estado salvo em `%APPDATA%\metro-notch\state.json` (apagar o arquivo volta ao padrão).

## 8. Conectividade (MCP + IPC)

| Código | Requisito |
|---|---|
| CN-01 | Servidor HTTP embutido em `127.0.0.1:7878` (override `NOTCH_PORT`), sem dependências novas (std::net). |
| CN-02 | Endpoint MCP `POST /mcp` (JSON-RPC 2.0) com tools de tarefas/config/estado. |
| CN-03 | Endpoints REST `/api/*` para integração inter-processo. |
| CN-04 | Mutações externas propagadas para a UI via `window.__load(...)`. |
| CN-05 | Documentação em `CONNECTIVITY.md` e adapters em `integrations/`. |

## 7. Arquivos

| Arquivo | Conteúdo |
|---|---|
| `src/main.rs` | Janela tao, WebView wry, estilo/WS_POPUP, vidro acrílico, IPC, persistência, topmost. |
| `src/ui.html` | Toda a interface: paletas, estados idle/alerta/lista/configurações, animações, arrasto, agendador. |
| `Cargo.toml` | Dependências (`tao 0.37`, `wry 0.57`, `serde_json 1`) e perfil release. |
