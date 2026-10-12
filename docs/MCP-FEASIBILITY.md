# Viabilidade MCP

Consulta oficial inicial em 07/10/2026 e guia Secure MCP Tunnel consultado novamente em 10/10. **Integração pelo plugin comprovada nesta conversa em 12/10/2026.** A ferramenta recallly_status está disponível e foi invocada diretamente pelo plugin Recallly Probe, sem simular a chamada pelo terminal. O resultado online foi confirmado nas quatro provas e no servidor Node original restaurado. Executar somente um cliente pelo terminal continua não sendo prova de conexão do chat.

## Conexão e requisitos

O [guia atual de servidor MCP próprio](https://developers.openai.com/api/docs/guides/custom-mcp-server) descreve ChatGPT web → Plugins → Add custom MCP server. A conexão vira um plugin que precisa ser instalado e selecionado com `@`. Permissões do workspace e restrições como Lockdown podem impedir acesso. SSE e streaming HTTP são aceitos na conexão remota. Não são exigidas ferramentas search/fetch.

O procedimento atual não exige explicitamente ativar Developer Mode. A opção de servidor MCP próprio e conexão Túnel foram observadas e usadas na conta de Thiago; nenhum passo separado de Developer Mode foi necessário. A opção ausente precisa ser resolvida nas permissões da conta/workspace.

O [Secure MCP Tunnel](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels) alcança MCP privado por stdio ou HTTP, usando conexão HTTPS de saída. Exige tunnel_id, chave de runtime, permissões Tunnels e associação ao workspace correto. Um token isolado não cria essa associação nem instala o plugin. A prova usa stdio; nenhuma porta é publicada. O túnel não atende submissão pública de plugins, que exige HTTPS público estável.

## Autenticação

O guia de servidor próprio aceita OAuth, ausência de autenticação ou modo misto. Escolha desta prova: sem autenticação de aplicação, exclusivamente diagnóstico anônimo. Isso é separado da chave que autentica o cliente do túnel.

O [guia de autenticação](https://developers.openai.com/plugins/build/auth) descreve OAuth 2.1, descoberta de metadados e PKCE S256; registro por CIMD, DCR ou cliente predefinido. Credenciais estáticas de cliente OAuth não equivalem a um token arbitrário. ChatGPT não apresenta API keys customizadas ou certificados mTLS fornecidos pelo cliente e não aceita grants OAuth máquina a máquina como client_credentials. mTLS gerenciado pela OpenAI identifica o cliente; OAuth autoriza o usuário.

## Evidência e limitações

Comprovado: inicialização MCP, descoberta da ferramenta única, chamadas válidas, schema de saída, erro controlado e encerramento local, conforme [VALIDATION](VALIDATION.md). Na segunda etapa, a Platform autenticada permitiu criar o túnel Recallly Probe e confirmar sua associação à organização e ao workspace disponível. Cliente oficial Windows e perfil stdio preparados. Após entrada protegida da chave pelo usuário, doctor retornou RESULT ok; run está ativo e healthz/readyz responderam HTTP 200. O plugin privado foi criado, instalado e aparece Conectado no ChatGPT web.

Em 12/10, chamadas reais de recallly_status pelo plugin retornaram os seis campos esperados com Node, Go, .NET e Rust. Após restaurar o perfil original Node, a chamada retornou online, versão 0.1.0, horário 2026-10-12T15:33:39.900Z e fuso America/Sao_Paulo. O identificador é UUID aleatório em memória, não identidade persistente da máquina. Houve timeouts transitórios na primeira chamada ao trocar para Go e ao restaurar Node; as repetições funcionaram, sem causa determinada. A instalação e o estado Conectado, isoladamente, continuam insuficientes para comprovar conexão.

O [guia de teste de plugins](https://developers.openai.com/plugins/deploy/connect-chatgpt) recomenda instalar o plugin, abrir nova conversa, selecionar com `@` e verificar a chamada real. Só esse teste permite declarar conexão com o ChatGPT. Computador e cliente precisam permanecer disponíveis; desligamento, suspensão, rede bloqueada, associação incorreta, ferramenta desabilitada e políticas de workspace podem impedir comandos.

Nenhuma promessa de execução futura ou em segundo plano foi validada. Esta etapa não implementa agendamento ou e-mail.
