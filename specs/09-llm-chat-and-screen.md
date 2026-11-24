# Spec 09 - Chat com LLM local + leitura de tela

## Origem (pedido explícito)
> "implemente um modulo nele para ter llm conversando chat e ler a tela antes
> pesquise na web como fazer leitura da tela para ele quero um modulo om baixo
> consumo de gpu 500mb no maximo (um parser de tela que leia o tetxo talvez usar
> estrategia do leitor de tela do windows e esse modelo deve usar"

## Pesquisa (o que foi verificado antes de implementar)
- Leitores de tela do Windows (Narrator) não fazem OCR: leem a árvore de
  **UI Automation (UIA)**. O caminho em Rust é o crate `uiautomation`
  (wrapper de `windows::Win32::UI::Accessibility`), com `TextPattern` para
  documentos e `Name`/`Value` para controles. Custo de GPU: zero.
- OCR/visão por pixel exigiria um modelo multimodal e VRAM muito acima do
  orçamento; foi descartado.
- Para o chat, o runtime local já instalado na máquina é o Ollama
  (API OpenAI-compatível em `127.0.0.1:11434`). Modelo pequeno em CPU
  (Q4, ~1.1 GB de pesos) usa **0 MB de VRAM**, dentro do teto de 500 MB.
  Offload parcial para GPU é opcional e explícito (env), porque o contexto
  CUDA sozinho já consome centenas de MB.

## Requisitos

### Leitura de tela (SC)
- SC-01: ler a **janela em primeiro plano** via UI Automation, sem screenshot
  e sem OCR.
- SC-02: quando o próprio Metro Notch está em foco, ler a última janela
  externa em foco (senão o contexto seria o próprio painel).
- SC-03: preferir o texto do documento (`TextPattern`); se não houver, coletar
  `Name` dos controles visíveis, com profundidade e orçamento de caracteres
  limitados (padrão 4000) para manter a latência baixa.
- SC-04: nunca ler campos marcados como senha (`is_password`).

### Chat (LC)
- LC-01: `POST /api/chat` recebe `{"messages":[{role,content}...]}` e devolve
  `{"reply": "..."}` usando LLM local.
- LC-02: por padrão o texto da tela é anexado como contexto de sistema;
  `"screen": false` desliga.
- LC-03: runtime configurável por env: `METRO_LLM_URL` (padrão
  `http://127.0.0.1:11434`), `METRO_LLM_MODEL` (padrão `lfm3:1.7b`),
  `METRO_LLM_CTX` (padrão 4096), `METRO_LLM_GPU_LAYERS` (padrão `0` = CPU,
  0 MB de VRAM).
- LC-04: `GET /api/llm` reporta runtime, modelo, se está disponível e os
  modelos instalados.
- LC-05: tools MCP `read_screen` e `chat` para as mesmas funções.

## Critérios de aceite
- AC-01: `GET /api/screen` devolve texto legível da janela ativa em < 1 s.
- AC-02: `POST /api/chat` responde usando o conteúdo da tela quando perguntado.
- AC-03: nenhuma dependência de GPU é obrigatória; VRAM padrão = 0 MB.
- AC-04: build release sem warnings novos.

## Rastreio
- Implementação: `src/screen.rs`, `src/llm.rs`, rotas em `src/http.rs`,
  `mod` em `src/main.rs`, dependência em `Cargo.toml`.
- Verificação: build release + `/api/screen` + `/api/chat` + `tools/list`.
