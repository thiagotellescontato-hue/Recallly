# Medições de arquitetura

Rodadas de 10 e 12/10/2026 no computador real: Windows 10 Pro 22H2, build 19045.6466, x64, quatro processadores lógicos e aproximadamente 7,9 GiB de RAM. Fuso detectado: America/Sao_Paulo. Sem coleta de hostname, usuário, IP ou identificador de hardware.

## Método

Cliente MCP oficial TypeScript 1.32.1, transporte stdio, mesmos seis campos e única ferramenta `recallly_status` em todos os candidatos. São verificados schema estrito, chamada válida, argumentos omitidos, estabilidade do UUID na sessão, argumentos extras, ferramenta inexistente e recuperação. Erros devem ser respostas MCP controladas; uma desconexão não conta como erro de validação esperado.

Cinco inicializações sequenciais por candidato. Tempo medido com relógio monotônico desde o início da conexão até a resposta initialize; inclui criação do processo, sem compilação. São partidas com cache do sistema já aquecido, não medições de boot frio. Somente a quinta execução recebe a medição de repouso: dez segundos de estabilização, depois 15 amostras a cada dois segundos. Compilações e instalações terminam antes da rodada final.

`Get-Process` fornece PrivateMemorySize64, WorkingSet64 e TotalProcessorTime. Toolhelp32 enumera descendentes. Cada amostra soma separadamente servidor, túnel, auxiliares e total antes de calcular medianas. CPU é a diferença de tempo de processo dividida pelo intervalo real, em percentual de um núcleo; nesta máquina, 100% representa um quarto da capacidade lógica. Também se registra o máximo observado. Zero em todos os intervalos significa abaixo da resolução observada, não consumo teoricamente nulo.

O cliente de teste e o coletor não fazem parte do total do aplicativo. Processos auxiliares de console fazem parte quando descendem da árvore medida. A soma do working set pode duplicar páginas compartilhadas, razão para priorizar memória privada. As medianas das categorias não precisam somar exatamente a mediana do total.

A rodada com túnel usa o mesmo cliente oficial 0.0.16 e perfil associado ao plugin existente. `--mcp.command` substitui o subprocesso apenas nessa execução, sem regravar perfil. Nenhum listener MCP é criado: transporte stdio. O listener administrativo do túnel fica exclusivamente em loopback. `/readyz` e a presença do subprocesso são conferidos. Uma chamada real pelo plugin é registrada separadamente e não é substituída por esses testes. Após cada árvore temporária encerrar, inicia-se a seguinte; ao final, restaura-se o perfil Node original.

## Ferramentas e builds

| Candidato | Build avaliado |
| --- | --- |
| Node | 24.19.0, ESM, SDK 1.32.1, zod 4.6.5, implementação original |
| Go | 1.27.1 windows/amd64, SDK 1.8.0, trimpath e símbolos removidos |
| .NET | SDK 10.0.401, runtime 10.0.12, MCP Core 2.2.0, win-x64 self-contained, single file e trimming |
| Rust | 1.99.0 GNU x64, rmcp 3.5.1, Tokio current-thread, release opt-level s, LTO, strip, panic abort |

Os comandos reproduzíveis e fontes estão em [bench](../bench/README.md). Os compiladores e caches foram preparados fora do checkout, em `work/architecture-work` da pasta desta conversa. Node, pnpm, Git e PowerShell já pertenciam ao runtime do Codex. Go e .NET foram extraídos de distribuições oficiais com checksum conferido. Rust foi montado a partir da distribuição portátil oficial; GNU binutils MSYS2 e suas dependências foram extraídos após conferência SHA-256 das páginas oficiais. Nenhuma mudança global de PATH ou de política de execução.

LLVM-MinGW portátil foi usado no diagnóstico da compilação Rust. Misturar seu wrapper GCC com o target GNU não encontrou as bibliotecas GCC; usar dlltool LLVM permitiu compilar, mas a chamada WinRT de detecção de fuso encerrou o processo com 0xC0000005. O diagnóstico com GNU dlltool também reproduziu essa falha. A prova foi corrigida para detectar a chave de fuso por GetDynamicTimeZoneInformation, como a prova Go. Não se atribui a falha ao protocolo MCP. Ambas as provas mapeiam somente o fuso deste computador e UTC, com erro explícito para outros fusos; não são implementações completas de calendário.

Uma tentativa adicional de Native AOT .NET foi bloqueada por ausência do linker C++. O instalador oficial Visual Studio Build Tools 2022, com assinatura Microsoft válida, instalou o Visual Studio Installer e seu serviço de elevação, mas as tentativas de instalar os componentes C++ encerraram com código 1 e erros de caminho/arquivo temporário ausente. Nenhum compilador C++ foi confirmado. A versão self-contained normal compilou e foi testada. Native AOT não recebe números estimados e não fundamenta a decisão. O restore do lockfile .NET terminou com aviso NU1900 por indisponibilidade da consulta NuGet de vulnerabilidades; isso não equivale a uma auditoria aprovada.

## Resultados

Medições locais finais, sem túnel. Memória em MiB; medianas de 15 amostras. CPU mediana e máxima foram 0,00% nos intervalos observados para todos os servidores e auxiliares. Inicialização: mediana de cinco partidas, em milissegundos.

| Candidato | Privada servidor | WS servidor | Privada auxiliares | Privada total local | Initialize ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Node | 30,79 | 54,05 | 6,20 | 36,99 | 823,04 |
| Go | 47,21 | 9,16 | 6,19 | 53,39 | 55,87 |
| .NET self-contained trimmed | 11,77 | 31,39 | 6,18 | 17,95 | 1091,59 |
| Rust | 0,70 | 3,61 | 6,19 | 6,88 | 38,23 |

As amostras e os cinco tempos individuais estão em [2026-10-10-standalone.json](../bench/results/2026-10-10-standalone.json). Node, Go e .NET foram coletados sequencialmente na mesma rodada. Rust foi coletado separadamente após corrigir a detecção de fuso e encerrar a compilação. Não houve provas simultâneas.

Espaço lógico de distribuição: Node 109.984.113 bytes, Go 8.256.512 bytes, .NET 15.381.833 bytes e Rust 1.106.432 bytes. Node inclui node.exe, dependências em node_modules e src; .NET inclui o runtime self-contained, excluindo PDB. Go e Rust dependem das DLLs de sistema Windows já instaladas; imports Rust foram conferidos no executável. Não se somam os SDKs de desenvolvimento à distribuição.

O executável de túnel necessário a stdio ocupa 22.591.488 bytes. O pacote oficial atualmente extraído ocupa 62.603.272 bytes, incluindo licenças, SBOM e cloudflared.exe de 39.751.680 bytes. Esse componente incluído no pacote não foi executado; nenhum serviço de túnel de terceiros foi configurado. Esses dois tamanhos são informados separadamente para não confundir o conjunto mínimo necessário com o espaço instalado nesta máquina.

### Rodada combinada concluída em 12/10/2026

Mesmo perfil, executável de túnel e plugin nas quatro provas. Cada candidato passou novamente cinco ciclos locais de initialize e validação; depois o túnel temporário iniciou, recebeu chamada real pelo plugin, estabilizou e foi medido. São 15 amostras por candidato, dois em dois segundos após dez segundos de estabilização. Nenhuma compilação ou instalação em paralelo. Métricas em MiB, medianas:

| Candidato | Privada servidor | Privada túnel | Privada auxiliares | Privada total | WS total | CPU total mediana / máxima % | Initialize ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Node | 30,64 | 54,61 | 6,16 | 91,45 | 102,28 | 0,00 / 0,00 | 750,02 |
| Go | 46,74 | 54,39 | 6,16 | 107,30 | 47,71 | 0,00 / 0,78 | 83,24 |
| .NET self-contained trimmed | 11,60 | 54,45 | 6,16 | 72,21 | 70,87 | 0,00 / 0,78 | 2357,04 |
| Rust | 0,72 | 54,60 | 6,17 | 61,48 | 42,42 | 0,00 / 0,00 | 98,49 |

CPU dos servidores e auxiliares foi 0,00% mediana e máxima; os picos Go/.NET ocorreram no processo do túnel. As amostras incluem WS de cada categoria e intervalos reais em [2026-10-12-tunnel.json](../bench/results/2026-10-12-tunnel.json). Tempos initialize são medianas de cinco partidas locais na rodada atual. Não são tempo de resposta remota. O `/readyz` do túnel foi observado uma vez por candidato em 671,95 ms, 548,75 ms, 541,61 ms e 543,19 ms, respectivamente; é um preflight distinto de initialize e de uma chamada MCP autenticada, sem mediana de readiness.

| Candidato | Distribuição servidor MiB | Servidor + executável de túnel MiB |
| --- | ---: | ---: |
| Node | 104,89 | 126,43 |
| Go | 7,87 | 29,42 |
| .NET | 14,67 | 36,21 |
| Rust | 1,06 | 22,60 |

O pacote de túnel completo extraído acrescenta 59,70 MiB, em vez dos 21,54 MiB do executável necessário a stdio. O tamanho instalado real dessa pasta é informado acima; o conjunto mínimo não presume remover arquivos da distribuição oficial. As ferramentas de desenvolvimento continuam separadas.

As quatro chamadas pelo plugin retornaram online, versão 0.1.0, fuso America/Sao_Paulo, horário atual e identificador aleatório de sessão. Horários UTC: Node 15:27:16.148, Go 15:28:58.608, .NET 15:30:28.936, Rust 15:31:54.277 em 12/10. A primeira chamada Go expirou; a segunda funcionou antes da coleta. Não foi determinada a causa. O plugin não foi reconfigurado. Depois da coleta, todas as árvores temporárias foram encerradas; o perfil Node original foi restaurado, readyz respondeu HTTP 200 e a chamada real retornou online às 15:33:39.900 UTC. A primeira tentativa após essa restauração também expirou e a segunda funcionou. Os timeouts não foram contados como sucesso.

Em 10/10, a primeira tentativa da rodada com túnel encerrou antes de readiness. O diagnóstico executado no terminal protegido do usuário retornou `control plane API key is malformed`; não houve medição combinada válida. O [validador da versão oficial 0.0.16](https://github.com/openai/tunnel-client/blob/v0.0.16/pkg/runtimeconfig/config.go) aceita somente letras ASCII, dígitos, `_` e `-`, sem espaços externos. Essa rejeição é local, antes da autenticação HTTP, e não comprova problema de permissão ou expiração.

Um teste isolado com texto fictício, sem credencial real, confirmou que `NetworkCredential` preserva o valor convertido de SecureString no Windows PowerShell 5.1.19041.6456. Outro teste com texto fictício de formato aceito retornou `CHECK control_plane_api_key PASS` e `RESULT ok`: portanto, doctor sozinho não verifica se a chave existe ou autoriza o túnel. Em 12/10, após orientar a colagem pelo botão direito no campo protegido, o usuário obteve formato aprovado e a rodada autenticada funcionou. Isso resolveu a entrada nesta sessão; não se afirma qual caractere foi recebido anteriormente. O segredo não foi acessado pelo agente nem persistido.

Validação repetida das fontes preparadas para revisão: pnpm check, pnpm test, node --check bench/run.mjs, build Go com -mod=readonly, publish .NET com lock e trimming, e modo validate do benchmark para os quatro candidatos passaram. A publicação .NET exigiu permitir o subprocesso MSBuild fora do sandbox; permaneceu o aviso NU1900 já descrito. Nenhum erro de compilação ficou pendente nessas variantes. Em 12/10 o modo tunnel repetiu a validação equivalente e concluiu as quatro medições; pnpm check e pnpm test foram executados novamente na revisão final. Native AOT permanece sem medição.

## Limites

Provas mínimas, um computador, uma janela curta de repouso por tecnologia e cinco partidas quentes. Sem banco, SMTP, calendário, carga concorrente, suspensão/retomada, atualização ou teste de longa duração. O consumo final após essas funcionalidades precisa de nova medição. O tamanho de distribuição inclui o runtime necessário ao usuário, mas exclui compiladores, caches de desenvolvimento e símbolos de depuração. Espaço lógico dos arquivos, não espaço alocado em clusters. A infraestrutura remota da OpenAI fica fora da medição local.

O baseline antigo de 07/10 tinha servidor Node e túnel em execução prolongada; não foi misturado com as partidas novas. Rodadas diagnósticas que ocorreram durante compilações não são usadas para escolher a arquitetura. Não foram usados números de desempenho de terceiros.
