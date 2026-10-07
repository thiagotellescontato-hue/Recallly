# Ambiente

Detectado em 07/10/2026, antes de implementar a prova:

| Item | Resultado |
| --- | --- |
| Sistema | Windows 10 Pro |
| Versão | 22H2 |
| Build | 19045.6466 |
| Arquitetura do sistema | X64 |
| PowerShell | 7.6.5 |
| Node.js | 24.19.0 |
| Git | 2.53.0.windows.3 |
| pnpm | 11.25.0 |
| Fuso Windows | E. South America Standard Time |
| Fuso detectado por Node.js | America/Sao_Paulo |

Versão obtida por leitura de `HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion`; arquitetura por `RuntimeInformation.OSArchitecture`. WMI/CIM negou acesso. Não foram coletados usuário, hostname, IP, e-mail ou identificadores de hardware.

Git, Node.js e pnpm foram usados do runtime fornecido pelo Codex; não se presume que estejam no PATH de um terminal externo. O comando Python no PATH apontava ao alias WindowsApps; isso não comprova Python instalado. npm, uv e dotnet não foram encontrados no PATH inicial. Python não foi necessário.

Dependências instaladas apenas para a prova: `@modelcontextprotocol/sdk` 1.32.1 e `zod` 4.6.5, com versões exatas em package.json e grafo em pnpm-lock.yaml. Não foram instalados serviços, túnel, regras de firewall nem inicialização automática.

Na etapa de publicação, Git Credential Manager 2.7.3 foi confirmado e o login oficial por dispositivo foi concluído para a conta proprietária. GitHub CLI não foi encontrado no PATH. Nenhuma credencial foi incluída no checkout.

Cliente oficial Secure MCP Tunnel 0.0.16 instalado de forma portátil fora do checkout, em `work/tools/secure-mcp/v0.0.16` da pasta desta conversa. Artefato Windows amd64 baixado da release `openai/tunnel-client`, SHA-256 conferido: `edef7241b0c647fcb30f1a80ff376b6b25c51927960f257a3f01e21b17c2aba6`. O executável respondeu a version, quickstart, init e doctor. O pacote também inclui cloudflared; ele não foi executado e nenhum túnel de terceiros foi configurado.

Perfil local criado em `work/secure-mcp/profiles`, fora do checkout, com referência `env:CONTROL_PLANE_API_KEY` e listener de saúde `127.0.0.1:0`. Não foram alterados PATH global, serviços, regras de firewall ou autostart.

Repositório oficial clonado via HTTPS: `https://github.com/thiagotellescontato-hue/Recallly.git`. Estado inicial vazio, sem commits; branch ativa `main`; remoto `origin` para essa URL, tanto fetch quanto push. A mensagem origin/main [gone] indicava ausência de commits remotos, não uma branch perdida com conteúdo. Nenhum AGENTS.md encontrado no checkout ou nos ancestrais aplicáveis.

O Git empacotado exigiu corrigir GIT_EXEC_PATH para localizar o helper HTTPS. Rede e subprocessos foram permitidos por execução escalada: tentativa inicial de clone falhou no proxy do sandbox e teste inicial falhou com spawn EPERM. Não foi necessário alterar o sistema.

O terminal interativo Windows PowerShell bloqueou scripts .ps1 por política de execução. O túnel foi iniciado por comandos diretos no console, sem alterar essa política. Cliente oficial ativo; healthz/readyz retornaram HTTP 200. Nenhuma credencial foi gravada no checkout.
