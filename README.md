# Recallly

Aplicativo local de lembretes em Rust, com ChatGPT como interface planejada via MCP.

Estado atual: versão 0.3.0, núcleo persistente SQLite schema 1 e ferramentas MCP para criar, listar, alterar e cancelar lembretes. A mesma instância de armazenamento atende toda a sessão, com idempotência, revisão otimista, transições de estado, recuperação no reinício e trava exclusiva por banco. Consulte [ferramentas MCP](docs/MCP-TOOLS.md) e [armazenamento](docs/STORAGE.md).

Ferramentas disponíveis: `recallly_status`, `recallly_create_reminder`, `recallly_list_reminders`, `recallly_update_reminder` e `recallly_cancel_reminder`. O destinatário autorizado é fixo, o fuso dos lembretes é `America/Sao_Paulo` e entradas desconhecidas são rejeitadas. Persistência não significa envio.

Não há scheduler, SMTP, envio de e-mails, credenciais de e-mail, autostart, interface gráfica ou interpretação de linguagem natural. Node 0.1.0 permanece como fallback de diagnóstico. A versão 0.3.0 foi validada localmente com bancos temporários, mas ainda não foi ativada por túnel ou validada pelo ChatGPT.

Comece por [PROBE-SETUP](docs/PROBE-SETUP.md). Consulte também [arquitetura](docs/ARCHITECTURE.md), [ambiente](docs/ENVIRONMENT.md), [viabilidade](docs/MCP-FEASIBILITY.md), [decisões](docs/DECISIONS.md) e [validação](docs/VALIDATION.md).
