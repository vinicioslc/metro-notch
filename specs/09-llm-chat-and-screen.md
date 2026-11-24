# Spec 09 - Chat com LLM local + leitura de tela + LFM3 Omni

## Origem (pedido explícito)
> "implemente um modulo nele para ter llm conversando chat e ler a tela antes
> pesquise na web como fazer leitura da tela para ele quero um modulo om baixo
> consumo de gpu 500mb no maximo (um parser de tela que leia o tetxo talvez usar
> estrategia do leitor de tela do windows e esse modelo deve usar"

Extensão:
> "implemente o lfm3 omni" (resposta do usuário: áudio transcrever/falar + trocar
> modelo padrão)

## Pesquisa (o que foi verificado antes de implementar)
- Leitores de tela do Windows (Narrator) não fazem OCR: leem a árvore de
  **UI Automation (UIA)**. O caminho em Rust é o crate `uiautomation`
  (wrapper de `windows::Win32::UI::Accessibility`), com `TextPattern` para
  documentos e `Name`/`Value` para controles. Custo de GPU: zero.
- OCR/visão por pixel exigiria um modelo multimodal e VRAM muito acima do
  orçamento; foi descartado.
- Runtime local: Ollama em `127.0.0.1:11434`. Modelo padrão: `lfm3-omni:latest`
  (1.7B Q4, ~2 GB pesos, caps: completion/vision/audio/speech/tools). Com
  `METRO_LLM_GPU_LAYERS=0` (padrão) roda em CPU, **0 MB de VRAM**, dentro do teto
  de 500 MB. Offload parcial para GPU é opcional e explícito.
- Áudio no omni: testado em Ollama 0.40.16. O endpoint nativo `/api/chat` não
  aceita `audio` de forma confiável; `/v1/chat/completions` com
  `content: [{type:"input_audio", input_audio:{data,format:"wav"}}]` transcreveu
  corretamente. Por isso a transcrição usa `/v1/chat/completions`.

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
  `http://127.0.0.1:11434`), `METRO_LLM_MODEL` (padrão `lfm3-omni:latest`),
  `METRO_LLM_CTX` (padrão 4096), `METRO_LLM_GPU_LAYERS` (padrão `0` = CPU,
  0 MB de VRAM).
- LC-04: `GET /api/llm` reporta runtime, modelo, se está disponível e os
  modelos instalados.
- LC-05: tools MCP `read_screen`, `chat` e `transcribe`.

### Áudio (AU)
- AU-01: `POST /api/transcribe` recebe `{"audio":"<base64>","format":"wav|webm|ogg"}`
  e devolve `{"text":"..."}`. `webm/ogg` são convertidos para wav 16k mono via
  `ffmpeg` antes do envio ao omni.
- AU-02: UI de chat tem botão de microfone; grava com `MediaRecorder`, envia
  para transcrição por IPC e preenche o campo de texto com o resultado.
- AU-03: "Falar" usa saída de voz nativa do sistema operacional (Web Speech
  Synthesis / SAPI via WebView), sem carregar modelo TTS extra.

## Critérios de aceite
- AC-01: `GET /api/screen` devolve texto legível da janela ativa em < 1 s.
- AC-02: `POST /api/chat` responde usando o conteúdo da tela quando perguntado.
- AC-03: nenhuma dependência de GPU é obrigatória; VRAM padrão = 0 MB.
- AC-04: build release sem warnings novos.

## Rastreio
- Implementação: `src/screen.rs`, `src/llm.rs`, rotas em `src/http.rs`,
  `mod` em `src/main.rs`, dependência em `Cargo.toml`.
- Verificação: build release + `/api/screen` + `/api/chat` + `tools/list`.
