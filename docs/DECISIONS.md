# Decisões desta etapa

- Node.js ESM e SDK MCP: runtime disponível, sem compilador e com cliente compatível para validar o protocolo. Escolha restrita à prova; tecnologia definitiva pendente.
- Transporte stdio: nenhum listener HTTP, porta, regra de firewall ou acesso público. Secure MCP Tunnel é o caminho oficial documentado para posterior teste privado.
- Única ferramenta `recallly_status`, schema estrito e metadados de somente leitura. Sem e-mails, alterações de arquivos ou comandos do sistema no handler.
- `machine_id` é UUID aleatório em memória, constante durante a execução e renovado ao reiniciar. Identifica anonimamente a máquina atendendo aquela sessão, sem hostname, hardware, usuário, IP ou hash reversível de informação pessoal. Identidade persistente entre reinícios fica pendente e não deve ser inferida desse UUID.
- Horário ISO 8601 em UTC e fuso IANA detectado pelo runtime, sem presumir o fuso informado pelo chat.
- Dependências fixadas e lockfile versionado. Credenciais e arquivos do túnel excluídos do Git; nenhum arquivo de exemplo contém segredo.
- Sem autenticação na ferramenta de diagnóstico; autenticação do transporte de túnel é separada. OAuth para funcionalidades futuras permanece pendente.
- Documentação atual de custom MCP descreve plugin no ChatGPT web; não foi imposto Developer Mode por inferência a partir de páginas antigas.
- Teste local não comprova conexão deste chat. Configuração da conta, túnel e chamada real pelo plugin permanecem pendentes de ação manual.
- Repositório inicial vazio; publicação solicitada diretamente em main, sem force push.

Fora do escopo: persistência de lembretes, agendador, SMTP, credenciais de e-mail, autostart Windows, interface final e distribuição pública.
