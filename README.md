# Recallly

Investigação de integração MCP própria com ChatGPT e execução local no Windows.

Estado atual: Recallly Probe 0.2.0 em Rust é a implementação principal, validada localmente no Windows x64. Única ferramenta: `recallly_status`, somente leitura, por stdio, sem parâmetros obrigatórios ou porta de rede. Node 0.1.0 permanece como fallback para reversão imediata. O plugin instalado e o perfil original foram preservados. Ativação do túnel e chamada real desta versão 0.2.0 estão pendentes da entrada protegida de uma chave válida; a conexão comprovada anteriormente usou versões 0.1.0.

Não há agendador, armazenamento de lembretes, envio de e-mails, SMTP, inicialização com Windows ou interface final. O aplicativo futuro ficará no servidor local, e ChatGPT será sua interface via ferramentas MCP restritas. Veja a [decisão de arquitetura](docs/ARCHITECTURE.md) e a [validação da migração](docs/VALIDATION.md). O consumo de 61,48 MiB com túnel pertence à prova comparativa anterior; a aplicação 0.2.0 foi medida localmente, sem túnel.

Comece por [PROBE-SETUP](docs/PROBE-SETUP.md). Consulte [ambiente](docs/ENVIRONMENT.md), [viabilidade](docs/MCP-FEASIBILITY.md), [decisões](docs/DECISIONS.md) e [validação](docs/VALIDATION.md).
