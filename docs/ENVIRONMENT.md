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

Repositório oficial clonado via HTTPS: `https://github.com/thiagotellescontato-hue/Recallly.git`. Estado inicial vazio, sem commits; branch ativa `main`; remoto `origin` para essa URL, tanto fetch quanto push. A mensagem origin/main [gone] indicava ausência de commits remotos, não uma branch perdida com conteúdo. Nenhum AGENTS.md encontrado no checkout ou nos ancestrais aplicáveis.

O Git empacotado exigiu corrigir GIT_EXEC_PATH para localizar o helper HTTPS. Rede e subprocessos foram permitidos por execução escalada: tentativa inicial de clone falhou no proxy do sandbox e teste inicial falhou com spawn EPERM. Não foi necessário alterar o sistema.
