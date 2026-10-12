# Comparação de arquitetura

## Implementação principal após a migração

Em 12/10/2026, a aplicação principal passou a ser Rust 0.2.0 na raiz do projeto: Cargo.toml, Cargo.lock, rust-toolchain.toml e src/main.rs, src/probe.rs e src/timezone.rs. bench/ continua exclusivamente comparativo. Node 0.1.0 foi preservado em src/server.mjs e seus testes continuam disponíveis. A migração não substituiu o subprocesso do túnel nem alterou o plugin; sua ativação depende da chave não disponível no ambiente do agente. A chamada remota 0.2.0 está pendente.

Dependências diretas têm versões exatas, incluindo rmcp 3.5.1, Tokio 1.53.2, windows-sys 0.61.2 e windows-timezones 0.5.1. Os bindings mantidos Windows substituem a estrutura FFI manual comparativa. GetDynamicTimeZoneInformation lê TimeZoneKeyName, independente do nome de exibição localizado; windows-timezones converte pela tabela Unicode CLDR para o identificador IANA representativo global. Não há mais uma lista limitada a São Paulo e UTC. Chave desconhecida, UTF-16 inválido ou falha nativa encerram a inicialização com mensagem genérica, sem presumir UTC nem expor dados locais. Aliases IANA e ambiguidades territoriais do mapeamento Windows continuam condições da tabela, não detecção de localização geográfica.

Fontes primárias: [API Windows](https://learn.microsoft.com/en-us/windows/win32/api/timezoneapi/nf-timezoneapi-getdynamictimezoneinformation), [bindings Microsoft](https://github.com/microsoft/windows-rs), [windows-timezones 0.5.1](https://docs.rs/windows-timezones/0.5.1/windows_timezones/enum.WindowsTimezone.html) e [origem CLDR da biblioteca](https://github.com/kyrias/windows-timezones). A aplicação lê apenas estado nativo necessário a horário, fuso e aleatoriedade; não abre portas, acessa rede, escreve arquivos ou implementa telemetria. Scripts de desenvolvimento e operação são separados do executável.

Consumo privado local mediano: servidor 0,71 MiB, auxiliar 6,25 MiB, total 6,96 MiB; sem regressão grosseira em relação à prova Rust anterior. Não se reaplica automaticamente o total combinado de 61,48 MiB à nova versão. Build, testes e limites estão em VALIDATION.md; comandos e reversão em PROBE-SETUP.md.

No produto futuro, servidor local permanentemente disponível manterá dados e executará os lembretes; ChatGPT será a interface principal por ferramentas MCP restritas de criação, consulta, alteração e cancelamento. Nenhuma dessas ferramentas ou persistência foi implementada. Após a validação real 0.2.0 pelo chat, o próximo passo será começar o núcleo persistente de lembretes, em uma etapa separada.

## Histórico da decisão

Decisão em 12/10/2026: escolher Rust para o aplicativo futuro, com SDK MCP oficial rmcp, transporte stdio e Secure MCP Tunnel oficial. As quatro provas passaram localmente e responderam a chamadas reais do plugin nesta conversa. Rust apresentou o menor total de memória privada, 61,48 MiB, incluindo túnel e auxiliares. A implementação Node existente e o plugin instalado foram preservados; o servidor original foi restaurado e respondeu pelo plugin ao final. Não houve migração do aplicativo.

## Compatibilidade e manutenção

A [lista oficial de SDKs MCP](https://modelcontextprotocol.io/docs/sdk), consultada em 10/10/2026, classifica TypeScript, Go, C# e Rust como Tier 1. Não há fundamento para descartar Rust ou Go por suposta ausência de SDK oficial. Foram usados [TypeScript SDK](https://github.com/modelcontextprotocol/typescript-sdk), [Go SDK](https://github.com/modelcontextprotocol/go-sdk), [C# SDK Core](https://github.com/modelcontextprotocol/csharp-sdk) e [rmcp](https://github.com/modelcontextprotocol/rust-sdk).

Todos passaram initialize, tools/list, chamada válida, argumentos omitidos, validação dos seis campos, rejeição controlada de argumentos extras e ferramenta desconhecida, recuperação e encerramento. O transporte usado é stdio, suportado pelo [Secure MCP Tunnel oficial](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels). A documentação confirma compatibilidade do transporte; a prova local não substitui a chamada pelo plugin.

| Opção | Distribuição Windows x64 | Manutenção e riscos observados |
| --- | --- | --- |
| Node atual | Runtime Node, fontes e dependências; SEA é outra possibilidade, não medida | Menor esforço de continuidade. Runtime e grafo npm acompanham o aplicativo. Integração Win32 exige ponte nativa adicional. |
| Go | Executável nativo; compilação portátil passou | SDK oficial e linguagem simples. Memória privada maior nesta prova. Mapeamento Windows/IANA da prova é limitado e precisa ser generalizado. SQLite sem CGO é uma opção; SMTP da biblioteca padrão está congelado. |
| .NET | Executável self-contained com trimming e runtime incluído; compilação passou | Integração Windows direta e SDK mantido com participação da Microsoft. Native AOT ficou bloqueado pela instalação do linker, não sendo usado como promessa de desempenho. |
| Rust | Executável nativo GNU x64; compilação e testes passaram | Menor memória privada local e menor distribuição entre as provas medidas. SDK oficial. Maior complexidade de compilação e revisão de FFI. Falha WinRT de fuso exigiu correção; mapeamento da prova ainda é limitado. |

O [SEA do Node](https://nodejs.org/api/single-executable-applications.html) e [Native AOT](https://learn.microsoft.com/en-us/dotnet/core/deploying/native-aot/) são alternativas de publicação, não resultados medidos desta rodada. Não foram atribuídas economias hipotéticas a elas.

Lockfiles preservam dependências. O lock npm contém 92 entradas de pacotes; o módulo Go declara o SDK e 11 módulos adicionais; Cargo.lock contém 73 pacotes além da própria prova, incluindo dependências de outros targets; o lock NuGet desta prova contém cinco pacotes, incluindo tarefa de build. Essas unidades não são equivalentes nem indicam quantidade de bibliotecas carregadas, vulnerabilidades ou consumo em execução. Recursos do SDK não usados foram desabilitados quando havia opção simples. Não foi concluída uma auditoria de vulnerabilidades; a consulta NuGet retornou NU1900 no ambiente restrito.

## Capacidades necessárias ao aplicativo futuro

Esta seção avalia viabilidade por documentação. Nenhuma das funcionalidades abaixo foi implementada ou medida.

| Capacidade | Node | Go | .NET | Rust |
| --- | --- | --- | --- | --- |
| Persistência local | node:sqlite ou binding SQLite | modernc.org/sqlite sem CGO, ou driver com CGO | Microsoft.Data.Sqlite | rusqlite; publicação bundled exige cadeia C compatível |
| Esperar próximo vencimento | Timer one-shot, reagendado ao mudar a fila | time.Timer | System.Threading.Timer | Tokio sleep_until |
| SMTP com TLS | Nodemailer, exigindo TLS | TLS explícito e cliente SMTP; avaliar substituto mantido para net/smtp | MailKit; SmtpClient não é recomendado para novos projetos | lettre com backend TLS selecionado explicitamente |
| Proteção de credencial no Windows | Ponte Win32/DPAPI a selecionar | Binding Win32/DPAPI | ProtectedData com escopo CurrentUser | windows-rs/FFI para DPAPI |
| Evitar duas instâncias | Ponte Win32 para mutex nomeado | Binding CreateMutexW | Mutex nomeado | Binding CreateMutexW |

SQLite atende persistência transacional local sem serviço de banco separado. Uma única espera pelo próximo vencimento evita consulta constante, mas precisa ser reavaliada após mudança de relógio, fuso, suspensão e retomada. A prova mínima não testa calendário ou durabilidade.

DPAPI protege o segredo vinculado ao contexto Windows; não protege contra outro processo já executando com acesso equivalente ao usuário. O futuro arquivo cifrado precisará de permissões locais apropriadas. Nenhuma credencial de e-mail foi solicitada ou usada. Um mutex nomeado pode impedir duas instâncias, mas isso ainda precisa ser integrado ao ciclo de vida do servidor stdio e do túnel.

Empacotamento e atualização devem distribuir artefatos Windows x64 versionados, com verificação de integridade, encerramento controlado antes da substituição e possibilidade de rollback. Compiladores e caches pertencem ao ambiente de desenvolvimento; não devem acompanhar a instalação do usuário. SQLite/TLS poderão acrescentar bibliotecas nativas e alterar os números. Não se decidiu por instalador, autoupdate ou inicialização automática nesta etapa.

## Fontes primárias das capacidades

- SQLite: [Node](https://nodejs.org/download/release/v24.19.0/docs/api/sqlite.html), [modernc.org/sqlite](https://pkg.go.dev/modernc.org/sqlite), [Microsoft.Data.Sqlite](https://learn.microsoft.com/en-us/dotnet/standard/data/sqlite/), [rusqlite](https://docs.rs/rusqlite/latest/rusqlite/) e [build do rusqlite](https://github.com/rusqlite/rusqlite).
- Esperas: [Go Timer](https://pkg.go.dev/time#Timer), [.NET Timer](https://learn.microsoft.com/en-us/dotnet/api/system.threading.timer?view=net-10.0) e [Tokio sleep_until](https://docs.rs/tokio/latest/tokio/time/fn.sleep_until.html).
- SMTP: [Nodemailer TLS](https://nodemailer.com/smtp), [net/smtp](https://pkg.go.dev/net/smtp), [MailKit](https://github.com/jstedfast/MailKit), [limitação de SmtpClient](https://learn.microsoft.com/en-us/dotnet/api/system.net.mail.smtpclient?view=net-10.0) e [lettre](https://docs.rs/lettre/latest/lettre/).
- Windows: [DPAPI](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata), [ProtectedData](https://learn.microsoft.com/en-us/dotnet/api/system.security.cryptography.protecteddata), [Mutex](https://learn.microsoft.com/en-us/dotnet/api/system.threading.mutex?view=net-10.0) e [bindings Microsoft windows-rs](https://github.com/microsoft/windows-rs).

## Decisão e concessões

Os números estão em [PERFORMANCE](PERFORMANCE.md), com amostras locais e combinadas versionadas sem dados pessoais. O total privado Rust ficou 29,96 MiB abaixo do Node (32,76%) e 10,73 MiB abaixo do .NET (14,85%). Working set e distribuição também foram menores. Todos tiveram CPU mediana abaixo da resolução observada; não se atribui vitória em CPU. A inicialização Rust foi rápida, embora Go tenha inicializado mais rápido na rodada de 12/10.

O túnel domina o total: 54,60 MiB dos 61,48 MiB na prova Rust. Não se promete reduzir esse custo por trocar a linguagem do servidor. O ganho contra .NET, isoladamente, exigiria cautela; aqui também houve distribuição muito menor, SDK oficial e sucesso real pelo mesmo plugin. Isso sustenta a escolha Rust apesar do custo maior de compilação e manutenção. .NET self-contained é a alternativa de reserva caso a validação das futuras dependências torne Rust impraticável; Native AOT permanece sem medição.

A arquitetura escolhida prevê um processo Rust Windows x64, rmcp stdio, SQLite local via rusqlite, uma espera Tokio pelo próximo vencimento, SMTP TLS via lettre, proteção DPAPI CurrentUser e mutex Windows nomeado. São escolhas de projeto, não funcionalidades implementadas. Usar bindings mantidos windows-rs para reduzir FFI manual e mapear fusos Windows/IANA de forma completa antes de calendário de produção. Não reutilizar como solução final o mapeamento limitado da prova.

Próxima etapa recomendada: preparar uma migração mínima para Rust mantendo somente recallly_status, validar build reproduzível Windows x64, fuso completo, testes e compatibilidade pelo plugin; só então autorizar substituição do servidor original. Avaliar separadamente a compilação SQLite bundled e o backend TLS, fixar dependências, testar durabilidade, suspensão/retomada e repetir o consumo após cada capacidade adicionada. Nenhuma credencial de e-mail deve entrar no repositório. Distribuição e atualizações devem incluir versão, integridade e rollback; assinatura, instalador e política de atualização continuam pendentes.

A confiança é suficiente para escolher a direção do projeto, limitada a uma prova mínima neste computador. Não é uma previsão de 61,48 MiB para o aplicativo completo nem comprovação de confiabilidade prolongada. Houve um timeout remoto inicial ao trocar para Go e outro ao restaurar Node; as repetições funcionaram. Não foi determinada a causa desses timeouts.
