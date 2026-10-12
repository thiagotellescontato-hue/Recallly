# Recallly

Investigação de integração MCP própria com ChatGPT e execução local no Windows.

Estado atual: Recallly Probe 0.1.0 implementado, validado localmente e publicado em `main`. Única ferramenta: `recallly_status`, somente leitura, sem parâmetros obrigatórios ou porta de rede. Conexão real pelo plugin comprovada em 12/10/2026 com Secure MCP Tunnel oficial. Após comparar Node, Go, .NET e Rust no Windows, Rust foi escolhido para o aplicativo futuro: menor memória privada total medida, 61,48 MiB incluindo túnel e auxiliares. A implementação Node e o plugin originais foram preservados e continuam operacionais.

Não há agendador, armazenamento de lembretes, envio de e-mails, SMTP, inicialização com Windows ou interface final. A migração para Rust ainda não foi implementada. Veja a [decisão de arquitetura](docs/ARCHITECTURE.md) e as [medições e limites](docs/PERFORMANCE.md).

Comece por [PROBE-SETUP](docs/PROBE-SETUP.md). Consulte [ambiente](docs/ENVIRONMENT.md), [viabilidade](docs/MCP-FEASIBILITY.md), [decisões](docs/DECISIONS.md) e [validação](docs/VALIDATION.md).
