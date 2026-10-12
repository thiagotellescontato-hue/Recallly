# Recallly

Aplicativo local de lembretes em Rust, com ChatGPT como futura interface de operação via MCP.

Estado atual: núcleo persistente em Rust com SQLite embutido, schema 1, idempotência, revisões, transições de estado, recuperação no reinício e proteção por banco contra duas instâncias. A API de lembretes é interna e testável; ainda não está exposta ao ChatGPT. Consulte [STORAGE](docs/STORAGE.md).

Única ferramenta MCP: `recallly_status`, versão 0.2.0, com os mesmos seis campos anônimos, sem dados do banco. A conexão real Rust 0.2.0 pelo plugin foi comprovada neste chat. O túnel ativo, seu executável e seu perfil foram preservados; o novo build com persistência é validado separadamente com bancos descartáveis. Node 0.1.0 permanece intacto como fallback de diagnóstico, sem persistência.

Não há agendador, envio de e-mails, SMTP, credenciais de e-mail, autostart, ferramentas MCP de lembretes, interface gráfica ou interpretação de linguagem natural. Os números de memória anteriores pertencem às provas sem armazenamento e não representam este núcleo.

Comece por [PROBE-SETUP](docs/PROBE-SETUP.md). Consulte [ambiente](docs/ENVIRONMENT.md), [viabilidade](docs/MCP-FEASIBILITY.md), [decisões](docs/DECISIONS.md) e [validação](docs/VALIDATION.md).
