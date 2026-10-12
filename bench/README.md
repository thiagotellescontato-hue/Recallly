# Comparação local no Windows x64

Estas provas são temporárias e não substituem `src/server.mjs`. Somente `recallly_status`, por stdio, sem banco, rede, agendamento ou e-mail. Os handlers não escrevem arquivos. O benchmark grava apenas números e nomes das tecnologias no arquivo de resultados escolhido.

## Compilar

Use as versões registradas em `docs/PERFORMANCE.md`. Execute a partir da raiz do checkout, com as ferramentas portáteis no PATH apenas da sessão. Os diretórios de cache e saída podem ficar fora do checkout.

```powershell
pnpm install --frozen-lockfile
pnpm check
pnpm test
New-Item -ItemType Directory -Force bench/out
go -C bench/probes/go build -trimpath -ldflags '-s -w' -o ../../out/recallly-go.exe .
dotnet publish bench/probes/dotnet/RecalllyComparison.csproj -c Release -o bench/out/dotnet -p:PublishTrimmed=true -p:RestoreLockedMode=true
cargo build --manifest-path bench/probes/rust/Cargo.toml --release --locked
```

Para a distribuição portátil Rust GNU usada nesta comparação, coloque GNU binutils e o diretório `rust/lib/rustlib/x86_64-pc-windows-gnu/bin/self-contained` no PATH da sessão e defina `RUSTFLAGS=-C link-self-contained=yes`. Não misture o GCC desse target com o wrapper GCC do LLVM-MinGW. Não são necessárias alterações no PATH global nem na política de execução do PowerShell.

## Configuração local

Crie `bench/local-config.json`, ignorado pelo Git. Os caminhos abaixo são relativos à raiz do checkout e devem ser ajustados ao local das ferramentas. Não inclua credenciais. `node` e `powershell` precisam localizar executáveis reais; no terminal externo as ferramentas do Codex podem não estar no PATH.

```json
{
  "node": "node.exe",
  "powershell": "pwsh.exe",
  "resultFile": "bench/local-results-standalone.json",
  "candidates": [
    { "name": "Node", "command": "node.exe", "args": ["src/server.mjs"] },
    { "name": "Go", "command": "bench/out/recallly-go.exe", "args": [] },
    { "name": ".NET self-contained trimmed", "command": "bench/out/dotnet/RecalllyComparison.exe", "args": [] },
    { "name": "Rust", "command": "bench/probes/rust/target/release/recallly-rust.exe", "args": [] }
  ]
}
```

```powershell
node bench/run.mjs bench/local-config.json standalone
```

Para executar somente as verificações MCP, sem amostrar memória ou iniciar túnel, use o modo `validate` e um arquivo de resultados separado:

```powershell
node bench/run.mjs bench/local-config.json validate
```

Cada candidato deve passar initialize/list/call, argumentos omitidos, argumentos inválidos, ferramenta inexistente e recuperação. São cinco inicializações; na quinta há estabilização de dez segundos e 15 amostras com intervalo de dois segundos. O processo é encerrado antes do candidato seguinte. Não execute compilações ou instalações em paralelo com as medições.

## Medir com o túnel oficial

O perfil oficial já associado ao plugin deve permanecer fora do checkout. Não crie outro túnel nem altere o plugin. Pare o daemon original antes da rodada; o benchmark recusa iniciar se encontrar `tunnel-client` ativo.

Acrescente à configuração local os campos `tunnel` (executável oficial), `profile`, `profileDir`, `tempHealthFile` (arquivo externo temporário), `originalHealthFile` (arquivo externo original) e ajuste `resultFile`. Para o modo túnel use caminhos absolutos nos comandos dos candidatos: o subprocesso do túnel pode ter outro diretório de trabalho.

A chave temporária Tunnels Read + Use deve existir em `CONTROL_PLANE_API_KEY` na sessão protegida do usuário. Nunca a coloque no JSON ou na linha de comando. O benchmark não imprime nem grava essa variável e a retira do ambiente usado nos testes locais e na coleta de métricas.

```powershell
node bench/run.mjs bench/local-config.json tunnel
```

A rodada usa `--mcp.command` para trocar somente o subprocesso durante o teste, sem salvar alterações no perfil. Confirma `/readyz` em loopback e mede a árvore de processos, distinguindo túnel, servidor, auxiliares e total. Encerra cada árvore temporária antes de iniciar a próxima. Ao finalizar, inclusive em erro, solicita reinício do túnel com o perfil original. Confirme separadamente a saúde desse daemon e uma chamada real pelo plugin; readiness não comprova uma chamada pelo ChatGPT.

Para repetir também a verificação pelo chat antes de cada coleta, acrescente caminhos externos `validationRequestFile` e `validationResponseFile`. O script escreve no primeiro o candidato e a fase `remote validation pending`, e aguarda até 60 segundos. Faça a chamada real pelo plugin; somente após confirmar os seis campos, escreva no segundo um JSON como `{"candidate":"Node","status":"passed"}`, usando exatamente o nome solicitado. Em falha, use `failed`. Sem resposta, registra `timed out`; nunca invente um sucesso. O script passa à fase sampling e espera dez segundos antes da coleta; não chame o plugin durante essa coleta. Esses arquivos não devem conter resposta completa, UUID, credenciais ou dados pessoais. Na rodada registrada de 12/10 essa verificação foi executada nas quatro provas.

Não interrompa a janela durante uma medição. Em caso de interrupção forçada, confira e encerre somente os processos pertencentes àquela rodada antes de restaurar o perfil original. O encerramento normal é gerenciado pelo script; o daemon original restaurado é mantido para o teste pelo chat.

## Limites

Valores de memória são medidos por processo e somados por amostra antes da mediana. Working set pode contar páginas compartilhadas mais de uma vez; memória privada é a principal comparação. CPU é percentual de um núcleo, não da capacidade total da máquina. Mediana zero não significa ausência absoluta de trabalho. Processos de console também entram no total quando são filhos da árvore medida. O processo de coleta e o cliente de teste ficam fora do total do aplicativo.

Go e Rust detectam a chave de fuso Windows, mas esta prova mapeia apenas `E. South America Standard Time` e `UTC`; outros fusos falham explicitamente. Não é uma implementação geral de calendário. Os UUIDs são aleatórios em memória e não são gravados nos resultados.
