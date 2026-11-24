# Instruções do projeto — Metro Notch

## Regra 1 — Todo pedido explícito vira spec

Toda solicitação explícita do usuário deve ser registrada como uma spec em
`specs/`, antes ou junto da implementação. A spec descreve o comportamento
acordado, os limites e a origem (verbatim do pedido quando útil).

Fluxo:
1. Criar/atualizar `specs/NN-slug.md` com: contexto, requisito, critérios de
   aceite e rastreio (código → implementação → verificação).
2. Implementar conforme a spec.
3. Atualizar `specs/README.md` (índice) e, quando fizer sentido,
   `REQUISITOS.md`.

Nenhum pedido explícito deve ficar só no chat: se não estiver em `specs/`, a
tarefa não está concluída.

## Regra 2 — Specs são fonte de verdade

Em caso de conflito entre código e spec, a spec vence; o código deve ser
corrigido ou a spec atualizada com a decisão do usuário.

## Regra 3 — Não inventar escopo

Implemente o menor conjunto que atende a spec. Funcionalidades não pedidas não
entram sem uma spec própria.