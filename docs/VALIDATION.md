# Validação realizada

## Ferramentas MCP Rust 0.3.0

Baseline: `main` e `origin/main` em `e9fa0be061c96fb6534dddbf4995ccceebce9251`, árvore limpa e remoto oficial conferido antes das alterações. O schema permaneceu na versão 1. Nenhum túnel, plugin, credencial, banco de produção ou envio de e-mail foi acessado.

O handler passou a possuir a única instância de Store, protegida por Mutex. Foram expostas `recallly_create_reminder`, `recallly_list_reminders`, `recallly_update_reminder` e `recallly_cancel_reminder`; `recallly_status` preservou os seis campos e passou a informar a versão 0.3.0. Schemas rejeitam propriedades desconhecidas e não aceitam destinatário, remetente ou timezone livre.

Os 25 testes Rust passaram: três do módulo de armazenamento, nove do binário incluindo sete testes do handler e dois de fuso, dois testes de processos e onze testes de integração do armazenamento. A cobertura nova inclui persistência, RFC 3339 com offset, conversão para São Paulo, limites em bytes, NUL/CR/LF, idempotência, todos os filtros de estado, ordenação, limites de listagem, atualizações parciais e múltiplas, no-op, revisão otimista, concorrência de revisões, estados de cancelamento, reinício e mensagens de erro sem detalhes internos.

O teste MCP real local iniciou o release 0.3.0 por stdio com banco temporário próprio. Conferiu as cinco ferramentas e schemas, executou criar, replay idempotente, listar, alterar, listar, cancelar, listar novamente e replay após cancelamento. Depois encerrou e reabriu o servidor sobre o mesmo banco temporário para confirmar persistência; ao final removeu o diretório. Também validou argumentos desconhecidos, tentativa de informar destinatário, erros categorizados, recuperação após erros, identidade por execução, stderr vazio e EOF normal. O teste Node 0.1.0 de fallback continuou passando.

Comandos finais, todos com exit code zero:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
cargo build --release --locked --offline
pnpm check
pnpm test
pnpm test:rust
```

O release Windows MSVC possui 3.352.064 bytes e SHA-256 `17028e3ce9eb2def76173ee262f84f06a75fb7009f9a1e38bff44e850d904895`. O rebuild offline utilizou somente o cache presente. Isso valida o artefato MSVC desta máquina; não afirma equivalência binária ou de distribuição com o build GNU anterior.

Todos os bancos usados eram temporários. Nenhum banco, WAL, SHM ou lock de aplicação ficou no checkout, e nenhum processo Recallly permaneceu ativo após os testes. Ainda não há scheduler, SMTP, envio real, credenciais de e-mail, autostart ou validação 0.3.0 pelo Secure MCP Tunnel/ChatGPT.

## Núcleo persistente Rust

Antes das alterações: checkout limpo em main no commit f8601e044c1f3f797dbb36d5f93f8af3cd7f2ee3, origin oficial conferido e nenhum AGENTS.md aplicável. cargo test --locked, build release, pnpm check, pnpm test e pnpm test:rust passaram. Os testes de subprocessos precisaram executar fora do sandbox por spawn EPERM; não houve mudança na política de execução do Windows.

Chamada real pelo plugin no início desta etapa retornou Recallly online, versão 0.2.0, UTC 2026-10-12T23:24:45.964Z e America/Sao_Paulo. Não se registra UUID, caminho ou credencial. O túnel e seu executável ativo foram preservados; o novo núcleo usa saída de build separada e bancos descartáveis. A chamada pelo plugin valida o artefato já ativo; a validação do núcleo persistente novo é local, sem alegação de troca operacional.

Rustfmt e Clippy foram ativados do pacote portátil Rust 1.99.0 já disponível. SQLite bundled compilou com o Clang/llvm-ar existentes, mantendo o linker GNU do Rust. Nenhuma instalação global, credencial, porta ou serviço adicional foi necessária.

Testes Rust: 18 testes passaram (três da biblioteca, dois de fuso, onze do armazenamento e dois de processos), cobrindo os 29 cenários solicitados em grupos. Incluem migration inicial, reabertura, versão futura/estrangeira/corrupção/schema inválido, rollback de DDL e versão, pragmas, constraints e índice, validações, idempotência original após edição/cancelamento, buscas, filtros/ordenação, revisão divergente, estados inválidos, vencimento exato, tentativas, conclusão, falhas temporária/definitiva e recuperação com relógio injetado. Oito conexões concorrentes, deliberadamente sem a trava do aplicativo, produziram uma única reivindicação e uma única tentativa.

Processos reais com banco temporário: primeira instância respondeu initialize e status; segunda falhou com mensagem genérica de instância ocupada e sem anunciar MCP; EOF encerrou a primeira com código zero; nova instância iniciou e preservou o conteúdo; término forçado liberou a trava para outra instância. Uma reivindicação antiga foi recuperada na inicialização, mantendo contador e invalidando a revisão anterior. Banco corrompido falhou antes de availability, sem expor caminho. Resposta de status manteve exatamente seis campos e não revelou o conteúdo sentinela do banco.

Todos os testes de armazenamento recebem caminho em TempDir; o cliente MCP cria e remove seu próprio diretório temporário, sobrescrevendo RECALLLY_DB_PATH apenas no filho de teste. Nenhum teste usa o banco de produção. O fallback src/server.mjs e test/probe.mjs não foram alterados. Alterações em probe.rs e timezone.rs são somente formatação Rust.

Revisão adicional confirmou rejeição de índice removido e de tabelas estrangeiras com nome parecido com o prefixo interno SQLite, preservação do erro definitivo após cancelamento e impossibilidade de reivindicar um cancelado mesmo depois de seu vencimento. Rustfmt, Clippy --all-targets --locked -- -D warnings e os 18 testes passaram após essa revisão. thiserror 2.0.21 foi preservado em relação ao SDK anterior, sem downgrade.

pnpm check e pnpm test passaram; a resposta Node foi suprimida na execução para não registrar UUID. A busca nos arquivos versionados não encontrou padrões de credenciais/chaves privadas, e-mails, caminhos pessoais ou UUIDs; nenhum banco, WAL/SHM, executável, target, log ou arquivo local está versionado. O exemplo histórico de UUID foi omitido. Conferência do caminho padrão confirmou que o banco de produção não existe; todos os testes usaram caminhos descartáveis explícitos.

No encerramento da revisão, healthz e readyz HTTP 200, um servidor original ativo e nenhum servidor novo de teste remanescente. Nova chamada real pelo plugin retornou online, versão 0.2.0, UTC 2026-10-12T23:53:09.931Z e America/Sao_Paulo. Nenhuma nova etapa manual de ChatGPT foi necessária. Perfil, configuração local do iniciador e plugin permaneceram sem alteração.

Build release --locked e rebuild release --locked --offline passaram. O rebuild offline recompilou o código final revisado usando somente o cache existente; não se alega reprodutibilidade entre máquinas ou builds limpos. Artefato final de 2.180.608 bytes, SHA-256 423d2c9308d082f7e1d4a9a4332ef0fa281b5fc1c79df81107e91a7f7a1974ba, fora do Git e separado do executável do túnel. pnpm test:rust passou contra esse release final com banco temporário, incluindo contrato completo, argumentos inválidos, ferramenta desconhecida, recuperação, renovação da identidade, stderr vazio e EOF com saída normal.

Não houve envio, SMTP, senha de aplicativo, scheduler, autostart, interface, interpretação de linguagem natural ou nova ferramenta MCP. Não há benchmark de memória deste núcleo nem teste de falha física de energia; backup, criptografia e ambiguidade de entrega futura permanecem pendentes.

As seções seguintes preservam o histórico anterior; estados de validação pendente e ausência de armazenamento referem-se àquelas etapas.

## Migração principal Rust 0.2.0 em 12/10/2026

Baseline antes de editar: pnpm install --frozen-lockfile, pnpm check e pnpm test passaram com Node 0.1.0. A chamada pelo plugin também confirmou Node 0.1.0 antes da interrupção. Na retomada, nenhum tunnel-client estava ativo, não havia health disponível nem CONTROL_PLANE_API_KEY no ambiente do agente. Não se tentou recuperar segredo do processo ou do usuário. O perfil Node externo foi preservado.

Comandos executados na implementação principal, com Rust 1.99.0 GNU x64, GNU binutils e caches externos:

```text
cargo test --locked
cargo build --release --locked
cargo build --release --locked --offline
pnpm install --frozen-lockfile
pnpm check
pnpm test
pnpm test:rust
```

Os dois testes unitários passaram: conversão CLDR de São Paulo, UTC, Los Angeles, Berlim, Tóquio e Índia; rejeição explícita de chave vazia/desconhecida. O teste MCP usa o cliente oficial TypeScript e executa duas sessões novas no artefato release. Conferiu initialize e versão 0.2.0, descoberta de exatamente recallly_status, inputSchema sem propriedades adicionais, outputSchema dos seis campos e anotações de leitura/não destrutiva/idempotente/sem mundo aberto, além de metadados noauth.

Chamadas com objeto vazio e argumentos omitidos passaram. service Recallly, status online, versão 0.2.0, ISO 8601 UTC atual, fuso America/Sao_Paulo e UUID v4 foram conferidos sem imprimir UUID. Texto JSON corresponde a structuredContent. UUID foi estável na sessão e diferente após nova execução. Propriedade extra, tentativa de definir fuso, ferramenta desconhecida, argumentos string e array falharam com erro MCP controlado; timeout ou desconexão não contam como rejeição válida. Nova chamada após os erros funcionou. stderr permaneceu vazio, os subprocessos foram encerrados e um teste separado confirmou saída de código 0 ao fechar stdin depois de initialize/initialized, sem depender de matar o servidor.

Build release com lockfile passou. Repetição incremental offline produziu o mesmo SHA-256: caf60d69ef7bb80b9a1663c5204cbb6eb08dfff9592eb8a1165af7572e30819a. Tamanho do executável: 1.021.440 bytes, aproximadamente 0,97 MiB. Isso confirma o rebuild neste ambiente e cache; não comprova identidade binária entre computadores ou builds limpos independentes. O executável não foi versionado.

Medição curta depois da compilação, sem túnel e sem compilar em paralelo: dez segundos de estabilização e 15 amostras a cada dois segundos, usando o mesmo coletor Windows da comparação. Memória privada mediana do servidor 0,7109375 MiB, auxiliar de console 6,25 MiB, total local 6,9609375 MiB. WS mediano servidor 3,64453125 MiB, total 14,37109375 MiB. CPU mediana e máxima 0,00% nos intervalos observados, não consumo teoricamente nulo. As [amostras anônimas](results/2026-10-12-rust-0.2.0-idle.json) não contêm UUID, PID, nomes de usuário, caminhos, IP ou identificador de túnel. Não houve regressão grosseira contra a prova Rust comparativa (0,70 MiB privados do servidor); não foi repetida a comparação entre quatro tecnologias. Não há medição combinada 0.2.0 com túnel.

Iniciador protegido: parsing PowerShell e sintaxe Node passaram. Sem chave, o helper rejeitou a operação de forma controlada antes de iniciar ou substituir processos. Teste autenticado de doctor/run/health/ready em Rust 0.2.0 não foi executado por falta da chave; o script preparado não é evidência desse teste. O túnel está parado, e Node permanece disponível e testado para reversão, sem alegação de estar online agora. O plugin não foi alterado. Ativação e chamada real 0.2.0 pelo ChatGPT continuam pendentes; o sucesso anterior do comparativo 0.1.0 não substitui essa validação.

Nenhum lembrete, banco, agendador, SMTP, e-mail, autostart ou ferramenta adicional foi implementado. Após a validação real da versão 0.2.0, o próximo passo será começar o núcleo persistente de lembretes em uma nova etapa.

## Histórico anterior à migração

## Atualização em 12/10/2026

Integração real comprovada nesta conversa por invocação direta do plugin Recallly Probe. As quatro provas comparativas passaram cinco ciclos MCP locais e chamadas reais pelo mesmo túnel. Depois das medições o servidor Node original foi restaurado: readyz HTTP 200 e recallly_status online, versão 0.1.0, horário 2026-10-12T15:33:39.900Z e fuso America/Sao_Paulo. Sem armazenamento do UUID de sessão nos resultados comparativos. Dois timeouts iniciais nas trocas Go e Node foram seguidos de sucesso; causa não determinada.

pnpm check e pnpm test repetidos na revisão final, incluindo argumentos inválidos, ferramenta desconhecida, recuperação e encerramento do subprocesso de teste. Resultados e método das quatro tecnologias em PERFORMANCE.md. Os registros abaixo preservam o histórico anterior, quando a chamada pelo chat ainda não havia sido comprovada.

Em 07/10/2026, Node.js 24.19.0 no Windows x64:

```text
pnpm check
pnpm test
```

Ambos concluíram com exit code 0 na execução final. A verificação usa `node --check` nos dois arquivos JavaScript. O teste usa `Client` e `StdioClientTransport` do SDK MCP 1.32.1, inicia o servidor como subprocesso real e faz initialize, tools/list e tools/call.

Resultado real da chamada local:

```json
{
  "service": "Recallly",
  "status": "online",
  "version": "0.1.0",
  "current_time": "2026-10-07T23:20:09.251Z",
  "timezone": "America/Sao_Paulo",
  "machine_id": "<omitido>"
}
```

- tools/list retornou somente recallly_status.
- Chamada com `{}` e chamada sem arguments funcionaram.
- Conteúdo textual JSON correspondeu ao structuredContent, com apenas os seis campos permitidos.
- UUID manteve-se igual em chamadas da mesma sessão.
- Parâmetro extra `{ "unexpected": true }` retornou isError true.
- Nome desconhecido retornou isError true.
- Nova chamada válida após os erros funcionou.
- stderr do servidor permaneceu vazio.
- client.close encerrou o processo; teste de existência pelo PID confirmou término.

Falhas iniciais do ambiente foram resolvidas: helper HTTPS do Git localizado por GIT_EXEC_PATH; clone com rede fora do sandbox; teste executado fora do sandbox após spawn EPERM. Nenhum erro de implementação foi encontrado nos testes finais.

O servidor não cria listener nem acessa filesystem, rede ou APIs externas em seu código. Nenhum túnel foi ativado. **Não houve chamada da ferramenta por um plugin instalado neste chat ou no ChatGPT.** Não há evidência de conexão ponta a ponta; os passos manuais estão em PROBE-SETUP.md.

## Segunda etapa em 07/10/2026

`pnpm check` e `pnpm test` passaram novamente após a publicação. Chamada local retornou `online`, versão `0.1.0`, horário `2026-10-07T23:34:30.859Z`, fuso `America/Sao_Paulo` e UUID anônimo. Erros controlados, recuperação e encerramento do servidor passaram. Revisão dos arquivos versionados e busca por padrões de tokens, chaves privadas e certificados não encontraram credenciais; essa busca não é uma garantia genérica de ausência de todo tipo de segredo.

Publicação confirmada por `git ls-remote origin refs/heads/main`: `d1cd0e5c429a1c826993d7adc533c756697a4caa`. [Commit original publicado](https://github.com/thiagotellescontato-hue/Recallly/commit/d1cd0e5c429a1c826993d7adc533c756697a4caa).

Na Platform, criação e associação do túnel confirmadas pela interface e pela reabertura do formulário de edição. Identificador: `tunnel_6ac6d763b35c8191bd9e8ae6ece0b62e`. Esse identificador não é uma credencial.

Cliente oficial Windows x64 0.0.16: checksum confirmado, execução de version/help, init e doctor. Primeira tentativa de init falhou porque o parser removeu barras invertidas do comando Windows; init com caminhos em `/` passou. Perfil externo e comando stdio corretos confirmados. `doctor` concluiu com `RESULT fail`, `FAILED_CHECKS control_plane_api_key`, `EXIT_CODE 2`, pois a variável de ambiente da chave não existe. A menção a plugin Codex opcional é SKIP, não motivo desse erro.

Um iniciador PowerShell externo ao checkout foi preparado, com parsing sintático sem erros. Ele recebe a chave com Read-Host -AsSecureString, usa apenas variável de ambiente, executa doctor, inicia run somente após diagnóstico aprovado e restaura o ambiente ao terminar. Não foi executado com chave nesta etapa.

Estado de prontidão: servidor MCP de teste encerrado; túnel cadastrado, mas daemon ainda não iniciado. Sem evidência de health/ready, MCP remoto ou chamada pelo chat. Não há porta pública ou encaminhamento de roteador.

## Ativação efetiva

Após entrada protegida da chave pelo usuário, doctor retornou RESULT ok. O comando run está ativo no terminal do usuário. Listener verificado exclusivamente em 127.0.0.1; healthz e readyz responderam HTTP 200. Plugin privado Recallly Probe criado e instalado pelo fluxo Túnel no ChatGPT web; interface mostra Conectado, autorização Nenhuma e status DEVELOPMENT.

Esses resultados substituem o estado anterior sem chave/daemon. Ainda não houve chamada real de recallly_status pelo chat. A ferramenta não aparece entre as ferramentas disponíveis desta sessão Codex. O túnel permanece ativo para o teste manual.

Verificação final repetida: pnpm check e pnpm test exit 0; chamada local online às 2026-10-07T23:52:15.015Z, fuso America/Sao_Paulo. Chamada inválida, ferramenta desconhecida, recuperação e encerramento passaram. A interface instalada não exibiu nesta verificação a lista ou resultado de tools/call remoto; não se apresenta essa evidência como comprovada.
