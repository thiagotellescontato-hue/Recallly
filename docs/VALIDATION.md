# Validação realizada

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
  "machine_id": "cdc6c622-a406-4075-b9de-2e6fe595b46b"
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
