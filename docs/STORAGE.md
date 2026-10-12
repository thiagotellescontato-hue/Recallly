# Núcleo persistente de lembretes

## Local e inicialização

Produção Windows: `%LOCALAPPDATA%\Recallly\reminders.sqlite3`. O caminho é absoluto, independente do diretório do terminal; a ausência de LOCALAPPDATA válida é erro, sem fallback para o checkout. O diretório é criado com as permissões herdadas do diretório local do usuário. Não se usa administrador, serviço ou mudança global de ACL.

Para desenvolvimento e testes, `RECALLLY_DB_PATH` substitui o caminho inteiro e deve ser absoluto. Nunca aponte testes para o banco real. Os testes Rust usam tempfile; o cliente MCP cria um diretório temporário próprio e remove-o ao terminar. Nenhum banco, WAL, SHM, lock ou conteúdo de lembrete entra no Git.

A inicialização resolve o caminho, adquire a trava, abre SQLite, configura timeout e foreign keys, verifica integridade, migra, habilita WAL e synchronous FULL, recupera reivindicações antigas e só depois inicia MCP. Erro de armazenamento impede initialize/availability. Mensagens de erro não contêm SQL, conteúdo ou caminhos pessoais.

SQLite bundled por rusqlite 0.40.2 elimina instalação de serviço ou DLL SQLite pelo usuário. Compilar exige um compilador C compatível com o target Rust. Nesta máquina foi usado o Clang LLVM-MinGW portátil existente, com arquivador llvm-ar; não se substituiu o linker GNU do Rust.

## Schema 1

`migrations/001_reminders.sql` contém a migration inicial. `PRAGMA user_version=1` identifica a versão; application_id identifica Recallly. O schema é STRICT, validado contra a migration embarcada; versão futura, banco de outro aplicativo, schema alterado e corrupção detectada por quick_check são rejeitados. DDL, application_id e user_version são aplicados na mesma transação IMMEDIATE, com rollback em erro. Bancos existentes não são apagados ou recriados.

Tabela `reminders`: id UUID; idempotency_key única; subject; message; scheduled_at_utc; timezone IANA; status; attempt_count; last_error opcional; created_at_utc; updated_at_utc; claimed_at_utc opcional; sent_at_utc opcional; revision. `original_content` guarda a representação canônica imutável da criação para idempotência. Não existe destinatário livre, remetente ou credencial no registro.

Instantes são inteiros de milissegundos desde Unix epoch, sempre absolutos em UTC. A API usa Instant, ReminderId, Timezone e Status tipados. Entrada RFC 3339 precisa conter offset, ter precisão máxima de milissegundos e ano UTC entre 1 e 9999; offsets são normalizados. Fusos são validados pela tabela IANA de chrono-tz 0.10.4. Não há conversão de horário local ambíguo nem interpretação de datas em linguagem natural nesta API.

Constraints verificam estado, contadores não negativos, revisão, limites de tamanho e coerência entre estado e timestamps. Índice `(status, scheduled_at_utc, id)` atende fila pendente e reivindicação; índice parcial claimed_at_utc atende recuperação. Timestamps não representam um agendador em funcionamento.

## API interna e validação

`src/storage` exporta Store e os tipos de domínio. Operações: create, get, get_by_key, list, upcoming, update, cancel, claim_next_due, mark_sent, temporary_failure, permanent_failure, recover_abandoned e counts. Clock é injetável; o servidor usa SystemClock. Operações de alteração usam transações IMMEDIATE. Leituras não expõem dados pelo MCP.

Limites em bytes UTF-8: assunto 200; mensagem 65.536; chave de idempotência 128; descrição de falha 2.048. Texto vazio, somente espaços ou contendo NUL é inválido; assunto também rejeita CR/LF. Chaves e conteúdo não são silenciosamente aparados ou truncados. Listagens aceitam limite de 1 a 1.000, ordenam por horário UTC e desempate por ID. upcoming inclui pendentes atrasados, para que nenhum vencido desapareça da fila. Listagens ainda não oferecem paginação por cursor.

Criação repetida com chave e conteúdo original iguais devolve o registro atual, inclusive após edição ou cancelamento. Reutilização da chave com outro assunto, mensagem, instante UTC ou identificador de fuso falha com IdempotencyConflict. Representações de offset do mesmo instante são equivalentes; aliases IANA diferentes são conteúdos diferentes. A chave permanece reservada: não há exclusão ou reciclagem nesta etapa.

Atualização exige conteúdo válido, estado pending e revisão exata. A API recebe o conteúdo completo; o chamador preserva os campos que não quer alterar. Toda mudança de estado/conteúdo incrementa revision. A revisão antiga é rejeitada, inclusive para finalizar uma reivindicação recuperada. O contador de tentativas não muda em criação, edição, cancelamento, falha ou recuperação.

## Estados e tentativas

| Origem | Operação | Destino |
| --- | --- | --- |
| novo | create | pending |
| pending vencido ou no instante devido | claim_next_due | claimed |
| claimed | mark_sent | sent |
| claimed | temporary_failure | pending |
| claimed | permanent_failure | failed |
| claimed com timeout atingido | recover_abandoned | pending |
| pending ou failed | cancel | cancelled |

sent e cancelled são terminais. Não é permitido editar failed ou cancelar uma tentativa claimed em andamento: primeiro ela precisa terminar ou ser recuperada. Isso evita um cancelamento concorrente com uma tentativa de envio futura. Falha definitiva permanece em failed e fora da fila; cancelá-la preserva last_error. Falha temporária preserva a descrição e reabre a fila, sem backoff automático. Ao concluir com sucesso, last_error é limpo e sent_at_utc é preenchido; claimed_at_utc existe somente em claimed.

Reivindicar significa iniciar uma tentativa de processamento: somente essa operação incrementa attempt_count, uma vez, na mesma transação que muda o estado. Um futuro executor deverá chamar claim_next_due apenas quando estiver pronto para começar a tentativa, nunca para simples consulta. Ainda não existe executor, SMTP ou tentativa real de envio nesta entrega.

Reivindicação seleciona o próximo pending com scheduled_at_utc <= Clock.now e o altera sob BEGIN IMMEDIATE. Teste com oito conexões concorrentes, deliberadamente sem a trava de instância, confirma uma única reivindicação. Não se depende apenas da proteção do processo.

## Reinício e recuperação

RECALLLY_CLAIM_TIMEOUT_SECONDS configura a recuperação inicial: padrão 900 segundos, faixa de 1 a 86.400. Reivindicações com idade igual ou superior ao limite voltam para pending, limpam claimed_at_utc, registram erro genérico e incrementam revision, sem incrementar tentativas. Reivindicações mais recentes permanecem claimed. A API também permite recuperação explícita com Duration positivo de pelo menos um milissegundo. Não há timer, polling ou recuperação periódica em segundo plano nesta etapa.

Uma tentativa antiga perde autoridade pela revisão. Isso protege a fila contra sobrescrita atrasada; não promete entrega de e-mail exatamente uma vez. No futuro, um crash após aceitação SMTP e antes de registrar sent exigirá política específica de ambiguidade e duplicação. A recuperação usa relógio UTC do sistema, sujeito a mudanças do relógio; não é prova de duração monotônica entre reinícios.

## Instância única e limites operacionais

Arquivo lateral `<banco>.lock`, vazio e sem PID, usa File::try_lock exclusivo da biblioteca padrão Rust. No Windows a trava é do sistema operacional e é liberada ao fechar o handle ou terminar o processo, inclusive à força. Store mantém o handle até depois de fechar SQLite. O arquivo lateral permanece vazio após o término: não o exclua enquanto houver qualquer instância ativa, pois isso criaria uma corrida.

O diretório pai é canonicalizado e o banco existente também. Caminhos equivalentes por `..` e nomes com diferenças de caixa no Windows compartilham a trava. Banco ou lock que seja symlink é rejeitado. Bancos diferentes têm travas independentes e testes paralelos não compartilham fila. Hard links, renomeação/substituição externa do banco ou do lock durante uso, compartilhamentos de rede e acesso por outro programa que ignore a trava não são suportados. Use disco local e um único caminho canônico; a proteção não é uma fronteira contra programas executando com acesso equivalente ao usuário.

foreign_keys ON, busy_timeout 5 segundos, WAL e synchronous FULL privilegiam integridade local. Falta de disco, falha de permissão ou corrupção retornam erro; não são corrigidas apagando o banco. WAL não substitui backup nem garante durabilidade contra hardware que não honre sincronização. Criptografia, backup/restauração, retenção, paginação e migrações posteriores continuam pendentes.

Fontes primárias: [rusqlite](https://docs.rs/rusqlite/0.40.2/rusqlite/), [SQLite WAL](https://www.sqlite.org/wal.html), [pragmas SQLite](https://www.sqlite.org/pragma.html), [File::try_lock](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock) e [chrono-tz](https://docs.rs/chrono-tz/0.10.4/chrono_tz/).

## Validação e execução

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
cargo build --release --locked --offline
pnpm check
pnpm test
pnpm test:rust
```

Configure o compilador conforme PROBE-SETUP antes dos comandos. Os testes de processo fazem initialize/status, concorrência de instâncias, EOF, término forçado, recuperação e reabertura com conteúdo persistido. Não iniciam o banco de produção, não usam chaves e não mexem no túnel. O contrato MCP mantém versão 0.2.0 e seis campos; o handler de status não lê nem escreve o banco, embora a inicialização do novo servidor abra o armazenamento.

O executável antigo do túnel em execução foi preservado. O novo núcleo é compilado em saída separada; ativá-lo operacionalmente será uma troca posterior controlada do artefato. Node continua apenas como fallback de diagnóstico e não substitui o armazenamento Rust.
