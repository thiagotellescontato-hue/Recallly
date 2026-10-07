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

## Continuação: publicação e preparação do túnel

- O commit original foi preservado e publicado sem alteração: `d1cd0e5c429a1c826993d7adc533c756697a4caa`. A autenticação foi feita pelo Git Credential Manager, via dispositivo, e o remoto confirmou o SHA em main.
- Túnel Recallly Probe criado na Platform, associado à organização e ao workspace disponível. Identificadores de organização/workspace ficam no perfil administrativo da Platform; não são necessários no código.
- Cliente oficial Windows x64 0.0.16 baixado e verificado fora do checkout. Perfil também externo, sem chave embutida e com listener de saúde apenas em loopback.
- O preflight do cliente descartou barras invertidas do comando Windows. Caminhos com `/`, mantendo aspas para espaços, passaram no init. As instruções foram corrigidas com essa evidência real.
- Formulário de chave de runtime preparado com somente Tunnels Read + Use e validade de um dia. A criação final e a entrada protegida da chave ficam com Thiago; nenhuma chave foi capturada ou gravada pelo agente.
- Não instalar o plugin opcional Tunnel MCP do Codex: ele não é necessário para a ferramenta de diagnóstico no ChatGPT. Não usar MCP stub ou Harpoon, pois acrescentariam ferramentas fora do escopo.
- Sem chave disponível no ambiente, não iniciar um daemon incapaz de autenticar nem apresentar o registro do túnel como runtime pronto. Ativação e teste real pelo chat permanecem pendentes.

Fora do escopo: persistência de lembretes, agendador, SMTP, credenciais de e-mail, autostart Windows, interface final e distribuição pública.

## Ativação e instalação

- Thiago criou a chave e inseriu o segredo em Read-Host protegido. Nenhuma chave foi lida ou gravada pelo agente.
- Windows PowerShell bloqueou execução do iniciador .ps1. Comandos diretos no console resolveram, sem mudar política de execução.
- Doctor RESULT ok e healthz/readyz HTTP 200 confirmados. Túnel mantido ativo no terminal do usuário para teste pelo ChatGPT.
- Plugin privado Recallly Probe criado e instalado no ChatGPT web, usando conexão Túnel e sem autenticação de aplicação. Interface confirma Conectado. Chamada real pelo chat ainda pendente.
