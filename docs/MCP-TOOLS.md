# Ferramentas MCP de lembretes

Recallly 0.3.0 expõe cinco ferramentas por stdio. `recallly_status` mantém o contrato anônimo de seis campos. As quatro ferramentas de lembretes usam a mesma instância de `Store` aberta na inicialização; não criam conexões por chamada nem ignoram a trava exclusiva do banco.

O destinatário é constante interna: `thiagotelles.contato@gmail.com`. O fuso dos lembretes é sempre `America/Sao_Paulo`. Nenhuma ferramenta aceita destinatário, remetente, timezone livre, SQL, coluna ou ordenação arbitrária.

## Criar

`recallly_create_reminder` exige `scheduled_at`, `subject`, `message` e `idempotency_key`. O instante deve ser RFC 3339 com offset explícito e precisão máxima de milissegundos. A resposta só é produzida depois do commit SQLite e contém o registro persistido, o horário UTC, a representação local e `idempotent_replay`.

Exemplo de entrada:

```json
{
  "scheduled_at": "2030-01-15T09:00:00-03:00",
  "subject": "Exemplo",
  "message": "Conteúdo de demonstração",
  "idempotency_key": "example-2030-01-15"
}
```

Criação repetida com a mesma chave e o mesmo conteúdo original devolve o registro atual com `idempotent_replay: true`. A chave reutilizada com conteúdo original diferente produz `idempotency_conflict`. O contrato omite `created`, pois seria apenas o inverso redundante de `idempotent_replay`.

## Listar

`recallly_list_reminders` aceita `status` e `limit` opcionais. Status válidos: `pending`, `claimed`, `sent`, `cancelled` e `failed`. O limite padrão é 50 e a faixa válida é 1 a 1.000. A ordenação é sempre por `scheduled_at_utc` e ID. Não há cursor nem paginação fictícia.

A resposta contém `recipient` uma vez no nível superior e `reminders`. Cada registro inclui ID, estado, revisão, horários UTC e local, fuso, assunto, mensagem, tentativas e timestamps de criação e atualização.

## Atualizar

`recallly_update_reminder` exige `id`, `expected_revision` e pelo menos um entre `scheduled_at`, `subject` e `message`. Campos omitidos preservam o valor atual. O conteúdo completo é validado antes da atualização.

Somente `pending` é editável. Revisão divergente produz `revision_conflict`. Uma requisição que não altera efetivamente o conteúdo produz `invalid_input` e não incrementa a revisão. Uma atualização válida incrementa a revisão exatamente uma vez.

## Cancelar

`recallly_cancel_reminder` exige `id` e `expected_revision`. O registro não é apagado. `pending` e `failed` podem virar `cancelled`; `claimed`, `sent` e `cancelled` são rejeitados com `incompatible_state` quando a revisão é atual. Repetir com uma revisão antiga produz `revision_conflict`.

A resposta contém ID, estado `cancelled`, nova revisão e `persisted: true`. Persistido significa apenas que a transação local foi confirmada.

## Estados e erros

`pending` significa armazenado e aguardando processamento. `claimed` significa que uma tentativa foi reivindicada internamente. `sent` somente poderá significar conclusão registrada depois que um executor de envio existir. A versão 0.3.0 não possui executor nem transforma automaticamente um lembrete persistido em `claimed` ou `sent`.

Erros controlados usam as categorias `invalid_input`, `not_found`, `idempotency_conflict`, `revision_conflict`, `incompatible_state` e `internal_error`. As mensagens não incluem SQL, stack trace ou caminhos locais.

Os limites continuam definidos pelo armazenamento: assunto de até 200 bytes UTF-8, mensagem de até 65.536 bytes e chave de idempotência de até 128 bytes. Valores não são aparados nem truncados. Texto vazio, somente espaços ou com NUL é inválido; assunto também rejeita CR e LF.

## Limitações atuais

Não há scheduler, SMTP, envio de e-mail, credenciais de e-mail, autostart, interface gráfica, interpretação de linguagem natural, paginação por cursor, backup ou criptografia. O Secure MCP Tunnel e o plugin não são configurados nem ativados por estas ferramentas. A validação desta versão é local; ativação real pelo ChatGPT é uma etapa separada.
