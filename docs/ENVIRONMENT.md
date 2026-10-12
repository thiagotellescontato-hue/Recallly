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

## Ferramentas da comparação de arquitetura

Preparadas até 10/10/2026, fora do checkout em `work/architecture-work` da pasta desta conversa:

| Ferramenta | Versão ou uso |
| --- | --- |
| Go portátil | 1.27.1 windows/amd64 |
| .NET SDK portátil | 10.0.401, runtime 10.0.12 |
| Rust e Cargo portáteis | 1.99.0, target x86_64-pc-windows-gnu |
| LLVM-MinGW portátil | release 20261006, diagnóstico de linking |
| GNU binutils MSYS2 portátil | pacote 2.47-4 e dependências de runtime verificadas |
| Windows PowerShell do usuário | 5.1.19041.6456, conversão SecureString testada com texto fictício |

Downloads oficiais com checksum conferido; SDKs, caches e executáveis não são versionados. Caminhos de ferramentas foram aplicados somente aos processos de compilação. A tentativa de rebuild Go encontrou bloqueio ao escrever o token de telemetria da ferramenta em sua configuração padrão; o rebuild passou com configuração Go isolada em `work/architecture-work/go-user-settings` e telemetria off, sem alterar a configuração original do usuário. Isso é separado dos servidores comparativos, que não implementam telemetria.

Exceção à instalação exclusivamente portátil: o instalador oficial Microsoft Build Tools 2022, com assinatura válida, instalou Visual Studio Installer e seu serviço de elevação. Os componentes C++ não foram confirmados: tentativas encerraram com código 1 e erros de arquivos temporários ausentes. Não se declara Native AOT disponível. Nenhum serviço ou inicialização automática do Recallly foi criado.

`pnpm check` e `pnpm test` passaram novamente em 10/10, incluindo encerramento do servidor. As quatro provas comparativas também passaram na validação MCP repetida. A medição combinada permanece bloqueada pela entrada malformada da chave no terminal protegido; o conteúdo da chave não foi lido pelo agente. O túnel não está ativo neste ponto da comparação.

## Estado após a rodada de 12/10/2026

O bloqueio de entrada foi superado com colagem pelo botão direito no prompt protegido. As quatro medições combinadas terminaram e cada prova respondeu pelo plugin instalado. Todas as árvores temporárias foram encerradas. O daemon original foi restaurado com perfil stdio Node preservado; readyz HTTP 200 e chamada real online confirmados. Somente o daemon original e seus filhos necessários permanecem para novas chamadas. A chave fica no ambiente do processo durante a execução; não foi persistida em arquivo nem acessada pelo agente. Sua validade e a disponibilidade do computador continuam condições para conexão.
