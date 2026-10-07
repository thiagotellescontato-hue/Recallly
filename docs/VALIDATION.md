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
