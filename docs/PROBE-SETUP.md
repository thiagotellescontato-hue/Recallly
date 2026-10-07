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

Preparação já concluída: túnel Recallly Probe criado e associação à organização e ao workspace disponível conferida. Identificador não secreto: `tunnel_6ac6d763b35c8191bd9e8ae6ece0b62e`. Cliente oficial Windows x64 0.0.16 baixado e checksum validado. Perfil stdio gerado e preflight do executável aprovado. Thiago criou e inseriu a chave em campo protegido. Doctor aprovado, run ativo e health/ready HTTP 200. Plugin privado instalado e conectado no ChatGPT web.

Na pasta desta conversa, o cliente fica em `work/tools/secure-mcp/v0.0.16` e o perfil em `work/secure-mcp/profiles`; ambos fora do checkout `work/Recallly`. Nenhum PATH global foi alterado. O arquivo externo `outputs/Start-RecalllyTunnel.ps1` recebe a chave com entrada protegida, faz doctor e só então inicia run, com health em loopback e sem gravar a chave.

### Iniciar no computador preparado

A chave foi criada pelo usuário com Tunnels Read + Use e validade de um dia. O terminal Windows PowerShell bloqueou o arquivo .ps1 por política de execução. Use os comandos diretamente no console; nenhuma política foi alterada ou contornada. Na pasta desta conversa:

```powershell
$probeSecret = Read-Host 'Chave de runtime do túnel' -AsSecureString
$env:CONTROL_PLANE_API_KEY = [System.Net.NetworkCredential]::new('', $probeSecret).Password
& '.\work\tools\secure-mcp\v0.0.16\tunnel-client.exe' doctor --profile recallly-probe --profile-dir '.\work\secure-mcp\profiles' --explain
```

Somente após RESULT ok, execute:

```powershell
& '.\work\tools\secure-mcp\v0.0.16\tunnel-client.exe' run --profile recallly-probe --profile-dir '.\work\secure-mcp\profiles' --health.url-file '.\work\secure-mcp\health.url' --log.level warn --mcp.stdio-send-initialized-notification
```

Mantenha o terminal aberto para o teste pelo ChatGPT. Insira o segredo apenas no campo protegido, nunca na linha de comando ou no chat. A existência de health.url sozinha não comprova prontidão; nesta execução healthz e readyz retornaram HTTP 200.

### Configuração reproduzível pelo cliente oficial

Para reproduzir o perfil fora do Git em outro checkout existente, baixe a [release oficial atual](https://github.com/openai/tunnel-client/releases/latest) indicada pelo [guia oficial](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels). Confirme checksum e arquitetura. Com o cliente no PATH, execute na raiz do checkout; o diretório de perfil é externo. Não sobrescreva o perfil preparado nesta máquina nem repita init sem necessidade.

```powershell
$probeTunnelId = Read-Host 'Identificador do túnel criado na Platform'
$probeNode = (Get-Command node).Source.Replace('\', '/')
$probeServer = (Resolve-Path 'src/server.mjs').Path.Replace('\', '/')
$probeCommand = '"' + $probeNode + '" "' + $probeServer + '"'
$probeProfileDir = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) '../secure-mcp/profiles'))
$probeHealthFile = Join-Path $probeProfileDir '../health.url'
tunnel-client help quickstart
tunnel-client init --sample sample_mcp_stdio_local --profile recallly-probe --profile-dir $probeProfileDir --tunnel-id $probeTunnelId --mcp-command $probeCommand --health-listen-addr 127.0.0.1:0
$probeSecret = Read-Host 'Chave de runtime do túnel' -AsSecureString
try {
    $env:CONTROL_PLANE_API_KEY = [System.Net.NetworkCredential]::new('', $probeSecret).Password
    tunnel-client doctor --profile recallly-probe --profile-dir $probeProfileDir --explain
    if ($LASTEXITCODE -ne 0) { throw 'Diagnóstico do túnel falhou.' }
    tunnel-client run --profile recallly-probe --profile-dir $probeProfileDir --health.url-file $probeHealthFile --log.level warn --mcp.stdio-send-initialized-notification
} finally {
    Remove-Item Env:CONTROL_PLANE_API_KEY -ErrorAction SilentlyContinue
    $probeSecret.Dispose()
    $probeSecret = $null
}
```

O parser do cliente 0.0.16 removeu barras invertidas da string de comando no primeiro teste; barras `/` e aspas passaram no init real. O cliente inicia o servidor stdio; não inicie uma segunda cópia manualmente. Nesta máquina, autenticação de runtime, doctor e run já foram testados com sucesso. Não ative MCP stub, Harpoon, plugin opcional de gestão do Codex, logs HTTP brutos ou acesso remoto à interface de saúde.

### Testar o plugin instalado

O plugin já está instalado: [Recallly Probe](https://chatgpt.com/plugins/plugin_asdk_app_6ac6dabb9f9c81919665052bb521dfa9). Mantenha o terminal aberto, clique em Testar no chat e confira a seleção do plugin antes de enviar o texto abaixo. Os passos seguintes documentam a instalação para outra sessão, sem necessidade de recriar o plugin atual.

1. No ChatGPT web, no workspace associado, abra Plugins → `+` → Add custom MCP server. Nome: Recallly Probe. Connection: Tunnel; selecione ou informe `tunnel_6ac6d763b35c8191bd9e8ae6ece0b62e`. Authentication: No authentication. Revise o aviso exibido e crie o plugin.
2. Confirme que a descoberta mostra somente `recallly_status`. Instale-o, abra uma conversa, selecione-o com `@` e envie exatamente: `Use Recallly Probe e chame recallly_status sem argumentos. Não use o terminal nem outras ferramentas para simular essa chamada.`
3. Confira a chamada real e os seis campos não sensíveis. Sem o plugin carregado nesta conversa, enviar o texto aqui não prova conexão e não autoriza substituir a chamada por teste no terminal.

Se a opção não existir ou o túnel não aparecer, verifique associação de workspace e Tunnels Read + Use. O [guia atual de custom MCP](https://developers.openai.com/api/docs/guides/custom-mcp-server) não impõe um passo separado de Developer Mode. Não exporte logs do túnel ou credenciais.

Para parar: Ctrl+C no terminal do túnel e confirme que o cliente e seu processo stdio encerraram. Limpe a variável temporária ou feche o terminal:

```powershell
Remove-Item Env:CONTROL_PLANE_API_KEY -ErrorAction SilentlyContinue
$probeSecret = $null
```

Não use exposição direta de porta, encaminhamento no roteador ou túnel de terceiros. Não grave perfis, certificados ou segredos no Git. O túnel está ativo e o plugin Recallly Probe foi instalado e conectado no ChatGPT web. Não houve registro da ferramenta nesta sessão Codex. Só uma chamada MCP real pelo chat comprova a integração.
