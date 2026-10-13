# Decisões desta etapa

## Ferramentas MCP 0.3.0

- Expor criar, listar, alterar e cancelar usando somente o Store existente e sem alterar o schema 1. O handler possui a única instância do armazenamento, protegida por Mutex, mantendo conexão e trava exclusivas durante toda a sessão.
- Fixar internamente `America/Sao_Paulo` e `thiagotelles.contato@gmail.com`; não aceitar timezone, remetente ou destinatário nas entradas.
- Retornar somente `idempotent_replay` na criação. Um campo `created` seria redundante e inverso. A confirmação significa commit SQLite, não envio.
- Listar no máximo 1.000 registros, com padrão 50, ordenação fixa por horário UTC e ID, filtros apenas pelo enum de estados e sem paginação até existir cursor seguro.
- Tratar atualização sem mudança efetiva como `invalid_input`, sem incrementar revisão. Revisão divergente tem precedência sobre a comparação de conteúdo; somente `pending` é editável.
- Manter as transições do núcleo no cancelamento. Repetição com revisão antiga produz conflito de revisão; repetição com a revisão atual de um registro já cancelado produz estado incompatível.
- Expor erros nas categorias estáveis `invalid_input`, `not_found`, `idempotency_conflict`, `revision_conflict`, `incompatible_state` e `internal_error`, sem detalhes de SQLite, caminhos ou stack trace.
- Preservar o runtime Tokio current-thread e o transporte stdio. Não adicionar HTTP, scheduler, SMTP, credenciais, autostart ou ativação de túnel.
- Validar localmente no target MSVC disponível. A equivalência de distribuição com o artefato GNU anterior permanece pendente e não é inferida dos testes MSVC.

## Núcleo persistente

- Implementar biblioteca interna Rust e schema SQLite 1, sem expor CRUD por MCP nesta entrega. Manter versão 0.2.0 e contrato de recallly_status; Node permanece intacto como fallback de diagnóstico.
- Produção em LOCALAPPDATA/Recallly/reminders.sqlite3; RECALLLY_DB_PATH absoluto apenas para desenvolvimento e testes. Nunca usar o diretório atual como fallback. Testes sempre em diretórios temporários.
- rusqlite 0.40.2 bundled, chrono-tz 0.10.4, thiserror 2.0.21 e tempfile 3.23.0 para testes, versões diretas fixas e lockfile. Preservar a versão thiserror já usada transitivamente pelo SDK. Usar compilador C portátil existente; ativar rustfmt e Clippy do pacote Rust já disponível, sem instalar serviços.
- WAL, synchronous FULL, foreign_keys ON e busy_timeout de cinco segundos. Migration transacional com user_version, application_id e conferência do schema embarcado. Banco incompatível/corrompido falha antes de MCP.
- File::try_lock exclusivo em arquivo lateral por banco, sem PID, mantido no disco e ignorado pelo Git. O sistema libera a trava após EOF, saída normal ou término forçado. Preservar o arquivo lateral evita corridas de exclusão. Hard links e alteração externa durante uso não são suportados.
- BEGIN IMMEDIATE mantém a reivindicação atômica mesmo sem a trava de instância. Revision impede atualização silenciosa e conclusão por tentativa antiga. Reivindicar é o início da tentativa; os demais passos não incrementam attempt_count.
- Idempotência compara conteúdo original imutável e devolve o registro atual quando a criação se repete. Cancelar claimed é rejeitado para evitar corrida com a tentativa futura; pending/failed podem ser cancelados. Estados terminais e regras completas em STORAGE.md.
- Recuperar na inicialização reivindicações com timeout atingido, padrão 900 segundos configurável. Não criar timer ou polling. Não prometer entrega exatamente uma vez em uma futura integração SMTP.
- Preservar túnel, plugin, perfil e executável ativo; construir a persistência em saída separada. A conexão real Rust 0.2.0 já foi comprovada pelo chat; não pedir nova ação manual para um contrato inalterado.
- Pendentes: CRUD MCP restrito, política de autorização, envio e ambiguidade de entrega, agendador, backup/restauração, criptografia, autostart e distribuição. Nenhuma credencial de e-mail foi solicitada ou usada.

## Histórico da prova inicial

- Node.js ESM e SDK MCP: runtime disponível, sem compilador e com cliente compatível para validar o protocolo. Escolha restrita à prova; tecnologia definitiva pendente.
- Transporte stdio: nenhum listener HTTP, porta, regra de firewall ou acesso público. Secure MCP Tunnel é o caminho oficial documentado para posterior teste privado.
- Única ferramenta `recallly_status`, schema estrito e metadados de somente leitura. Sem e-mails, alterações de arquivos ou comandos do sistema no handler.
- `machine_id` é UUID aleatório em memória, constante durante a execução e renovado ao reiniciar. Identifica anonimamente a máquina atendendo aquela sessão, sem hostname, hardware, usuário, IP ou hash reversível de informação pessoal. Identidade persistente entre reinícios fica pendente e não deve ser inferida desse UUID.
- Horário ISO 8601 em UTC e fuso IANA detectado pelo runtime, sem presumir o fuso informado pelo chat.
- Dependências fixadas e lockfile versionado. Credenciais e arquivos do túnel excluídos do Git; nenhum arquivo de exemplo contém segredo.
- Sem autenticação na ferramenta de diagnóstico; autenticação do transporte de túnel é separada. OAuth para funcionalidades futuras permanece pendente.
- Documentação atual de custom MCP descreve plugin no ChatGPT web; não foi imposto Developer Mode por inferência a partir de páginas antigas.
- Teste local não comprova conexão deste chat. Configuração da conta, túnel e chamada real pelo plugin permanecem pendentes de ação manual.
- Repositório inicial vazio; publicação solicitada diretamente em main, sem force push.

## Continuação: publicação e preparação do túnel

- O commit original foi preservado e publicado sem alteração: `d1cd0e5c429a1c826993d7adc533c756697a4caa`. A autenticação foi feita pelo Git Credential Manager, via dispositivo, e o remoto confirmou o SHA em main.
- Túnel Recallly Probe criado na Platform, associado à organização e ao workspace disponível. Identificadores de organização/workspace ficam no perfil administrativo da Platform; não são necessários no código.
- Cliente oficial Windows x64 0.0.16 baixado e verificado fora do checkout. Perfil também externo, sem chave embutida e com listener de saúde apenas em loopback.
- O preflight do cliente descartou barras invertidas do comando Windows. Caminhos com `/`, mantendo aspas para espaços, passaram no init. As instruções foram corrigidas com essa evidência real.
- Formulário de chave de runtime preparado com somente Tunnels Read + Use e validade de um dia. A criação final e a entrada protegida da chave ficam com Thiago; nenhuma chave foi capturada ou gravada pelo agente.
- Não instalar o plugin opcional Tunnel MCP do Codex: ele não é necessário para a ferramenta de diagnóstico no ChatGPT. Não usar MCP stub ou Harpoon, pois acrescentariam ferramentas fora do escopo.
- Sem chave disponível no ambiente, não iniciar um daemon incapaz de autenticar nem apresentar o registro do túnel como runtime pronto. Ativação e teste real pelo chat permanecem pendentes.

Fora do escopo: persistência de lembretes, agendador, SMTP, credenciais de e-mail, autostart Windows, interface final e distribuição pública.

## Ativação e instalação

- Thiago criou a chave e inseriu o segredo em Read-Host protegido. Nenhuma chave foi lida ou gravada pelo agente.
- Windows PowerShell bloqueou execução do iniciador .ps1. Comandos diretos no console resolveram, sem mudar política de execução.
- Doctor RESULT ok e healthz/readyz HTTP 200 confirmados. Túnel mantido ativo no terminal do usuário para teste pelo ChatGPT.
- Plugin privado Recallly Probe criado e instalado no ChatGPT web, usando conexão Túnel e sem autenticação de aplicação. Interface confirma Conectado. Chamada real pelo chat ainda pendente.

## Comparação de arquitetura em 10/10/2026 — em andamento

- Construir provas equivalentes Go, .NET e Rust, mantendo Node e plugin originais. Nenhuma funcionalidade do aplicativo futuro foi implementada.
- Comparar cinco inicializações e 15 amostras de repouso por candidato, separando servidor, túnel, auxiliares e total. Não misturar resultados diagnósticos coletados durante compilações com a rodada final.
- Usar SDKs oficiais, todos atualmente Tier 1. Compiladores e artefatos ficam fora do checkout; versionar somente fontes, lockfiles, método e métricas sem identificadores pessoais.
- .NET self-contained com trimming compilou. Native AOT não foi medido por bloqueio da instalação do linker Microsoft; não estimar seu consumo.
- Corrigir a falha da prova Rust em detecção WinRT do fuso com leitura da API nativa Windows. Mapeamento Go/Rust desta prova cobre somente o fuso local e UTC; suporte geral precisa ser resolvido antes de calendário de produção.
- A primeira tentativa de medir com túnel falhou antes de readiness por chave malformada. O segredo real permanece exclusivamente no terminal do usuário. Testes com texto fictício confirmaram a conversão SecureString no PowerShell 5.1 e demonstraram que doctor aprova somente formato, sem provar autenticação.
- Preparar entrada protegida com classificação de erro sem exibir o valor, fora do checkout. Não insistir em criar novas chaves sem distinguir entrada abreviada, aspas, espaços e outros caracteres inválidos.
- Enquanto o usuário está AFK, não iniciar fluxo que dependa da chave real. Processos de provas temporárias estão encerrados. Túnel original ainda precisa ser restaurado após resolver a entrada da chave.
- Decisão final, atualização do estado no README, medição combinada, validação do plugin e commit `docs: select Recallly architecture` permanecem pendentes. Não publicar a comparação parcial como conclusão.

## Decisão concluída em 12/10/2026

- Escolher Rust com rmcp e stdio para o aplicativo futuro. Total privado medido com túnel e auxiliares: 61,48 MiB, menor entre as quatro provas; SDK oficial e chamada real pelo mesmo plugin confirmados. Justificativas, concessões e limites em ARCHITECTURE.md e PERFORMANCE.md.
- Manter Node como implementação operacional nesta etapa. Não alterar o plugin nem salvar a troca de subprocesso no perfil. As quatro árvores temporárias foram encerradas e o perfil original foi restaurado, com readyz HTTP 200 e chamada real online às 2026-10-12T15:33:39.900Z.
- Prever rusqlite, Tokio, lettre, DPAPI e mutex Windows no projeto futuro, sem implementar esses recursos nesta etapa. Resolver o mapeamento completo de fusos e reduzir FFI manual usando bindings mantidos antes de substituir o servidor original.
- Aceitar maior complexidade de build Rust em troca do menor consumo observado e da distribuição menor. .NET self-contained é alternativa de reserva; Native AOT não foi medido. O consumo com SQLite e TLS precisará ser medido novamente.
- Formato da chave aprovado após colagem pelo botão direito no campo protegido; segredo não lido nem persistido. Doctor não prova autenticação. A autenticação e a conexão foram comprovadas pelas chamadas reais, separadamente.
- Registrar os dois timeouts iniciais nas trocas Go e Node, seguidos de sucesso. Não atribuir causa sem evidência nem tratar falhas como validação aprovada.
- Versionar apenas fontes comparativas, locks, método, documentação e números anônimos. Compiladores, executáveis, perfis, chaves e resultados locais com caminhos continuam externos e ignorados.
- Próxima etapa: migração mínima revisável para Rust com somente recallly_status, fuso completo e build Windows reproduzível. Armazenamento, agendador, SMTP, autostart, interface final, assinatura e atualização automática permanecem fora desta entrega.

## Migração da prova para Rust 0.2.0

- Implementação principal na raiz Cargo/src; não promover bench/ a aplicação. Versão 0.2.0 no handshake e resposta da única ferramenta recallly_status. Manter os seis campos, anotações de leitura e metadados noauth.
- Fixar Rust 1.99.0, dependências diretas exatas e Cargo.lock. Publicação Windows GNU x64 release com LTO, símbolos removidos e sem privilégio administrativo. Build e caches ficam fora do checkout.
- Substituir FFI manual pelo binding windows-sys; converter a chave Windows pela tabela CLDR de windows-timezones. Não usar nome localizado, lista de dois fusos nem WinRT usado no diagnóstico comparativo. Falhas desconhecidas não devem virar UTC silenciosamente.
- Preservar integralmente src/server.mjs como fallback Node 0.1.0 e manter seus testes. pnpm test continua validando Node; pnpm test:rust valida o executável principal. Node/pnpm são ferramentas auxiliares, não runtime da aplicação Rust.
- Usar UUID v4 aleatório em memória por execução, estável na sessão e diferente após reiniciar. Não gravar UUIDs nos novos resultados ou saídas de validação Rust.
- Preparar Connect-Probe.cmd e tunnel.mjs para validação local, doctor, troca temporária via --mcp.command, health/ready e reversão Node. Não salvar a troca no perfil, recriar plugin ou recuperar credenciais de processos.
- Na retomada, nenhum daemon estava ativo, health não estava disponível e a chave não existia no ambiente do agente. Não foi possível ativar Rust nem restaurar um daemon Node autenticado sem participação do usuário. Perfil e comando Node foram preservados; não apresentar fallback disponível como conexão online.
- Testes locais e medição curta Rust concluídos. A prova comparativa 0.1.0 respondeu pelo chat anteriormente; a validação ponta a ponta da aplicação 0.2.0 permanece pendente. Publicar a implementação localmente testada sem inventar essa evidência.
- Manter credenciais, configuração local, executáveis, target e caches fora do Git. Próximo passo após validar 0.2.0 pelo chat: começar o núcleo persistente de lembretes, sem SMTP, agendador ou outras funcionalidades nesta entrega.
