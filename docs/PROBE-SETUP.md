# Executar Recallly Probe

## Execução local

Use PowerShell na raiz do checkout. Requer Node.js 22 ou superior e pnpm 11.25.0 disponíveis no PATH. Não é necessária chave OpenAI para o teste local.

```powershell
pnpm install --frozen-lockfile
pnpm check
pnpm test
```

`check` faz a verificação sintática, sem etapa de compilação. `test` inicia o servidor, conecta um cliente do SDK MCP, lista e chama a ferramenta, verifica erros e encerra o processo automaticamente.

Para iniciar manualmente:

```powershell
node src/server.mjs
```

O servidor espera mensagens MCP no stdin; silêncio no terminal é esperado. Pare com Ctrl+C. Um cliente stdio deve iniciar `node` com o argumento absoluto de `src/server.mjs`; não envie texto livre ao stdin. Há somente `recallly_status`, com argumentos `{}` ou omitidos. Propriedades extras são rejeitadas.

## Conectar ao ChatGPT sem publicar porta

Esses passos dependem da conta de Thiago e ainda não foram executados. Siga o [procedimento oficial Secure MCP Tunnel](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels):

1. Nas [configurações de túneis da Platform](https://platform.openai.com/settings/organization/tunnels), crie o túnel e associe a organização e o workspace alvo. Obtenha permissões Read + Manage para criar e Read + Use para executar/selecionar.
2. Baixe o cliente oficial pelo link da Platform ou pela [última release oficial](https://github.com/openai/tunnel-client/releases/latest). Escolha o artefato compatível com Windows x64; disponibilidade do binário não foi testada aqui. Coloque-o fora do repositório e deixe `tunnel-client` acessível no PATH.
3. Configure a chave de runtime somente no ambiente temporário e prepare o perfil com os comandos abaixo. A entrada protegida evita gravar o segredo no histórico. Execute na raiz do checkout. Não cole a chave no chat ou em arquivos.

```powershell
$probeSecret = Read-Host 'Chave de runtime do túnel' -AsSecureString
$env:CONTROL_PLANE_API_KEY = [System.Net.NetworkCredential]::new('', $probeSecret).Password
$probeTunnelId = Read-Host 'Identificador do túnel criado na Platform'
$probeNode = (Get-Command node).Source
$probeServer = (Resolve-Path 'src/server.mjs').Path
$probeCommand = '"' + $probeNode + '" "' + $probeServer + '"'
tunnel-client help quickstart
tunnel-client init --sample sample_mcp_stdio_local --profile recallly-probe --tunnel-id $probeTunnelId --mcp-command $probeCommand
tunnel-client doctor --profile recallly-probe --explain
tunnel-client run --profile recallly-probe
```

Os comandos init/doctor/run seguem a documentação; execução, parsing de caminhos Windows com espaços e saúde do túnel precisam ser confirmados com o cliente oficial baixado. Não declare sucesso se doctor indicar falha. O cliente inicia o servidor stdio; não é necessário iniciar outro servidor manualmente.

4. No ChatGPT web, abra Plugins → `+` → Add custom MCP server. Nome: Recallly Probe. Connection: Tunnel; selecione o túnel. Authentication: No authentication. Revise o aviso exibido e crie o plugin. Instale-o, abra nova conversa, selecione-o com `@` e peça: `Use Recallly Probe e chame recallly_status sem argumentos.`
5. Confira a chamada real e o resultado. Registre apenas os seis campos não sensíveis e o estado da conexão; não exporte logs do túnel ou credenciais. Se a opção não existir, resolva acesso com o administrador do workspace. O [guia atual](https://developers.openai.com/api/docs/guides/custom-mcp-server) não impõe um passo separado de Developer Mode.

Para parar: Ctrl+C no terminal do túnel e confirme que o cliente e seu processo stdio encerraram. Limpe a variável temporária ou feche o terminal:

```powershell
Remove-Item Env:CONTROL_PLANE_API_KEY -ErrorAction SilentlyContinue
$probeSecret = $null
```

Não use exposição direta de porta, encaminhamento no roteador ou túnel de terceiros. Não grave perfis, certificados ou segredos no Git. Esta etapa não instala automaticamente o cliente nem altera configurações do ChatGPT/Codex.
