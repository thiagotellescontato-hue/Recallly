# Executar Recallly Probe

## Prova principal Rust 0.2.0

Execute a partir da raiz do checkout no Windows x64. A aplicação usa somente stdio e expõe recallly_status, sem banco, rede, arquivos ou telemetria. O processo espera um cliente MCP; silêncio no terminal é normal. Node 0.1.0 continua em src/server.mjs para reversão.

Rust 1.99.0 e dependências diretas fixas estão em rust-toolchain.toml e Cargo.toml. Cargo.lock fixa também as dependências transitivas. Não é necessária chave OpenAI para compilar ou testar localmente. Nesta máquina, as ferramentas portáteis já preparadas ficam fora do checkout. Configure somente esta sessão PowerShell:

```powershell
$tools = [IO.Path]::GetFullPath((Join-Path (Get-Location) '../architecture-work'))
$env:PATH = "$tools/msys2/mingw64/bin;$tools/rust/bin;$tools/rust/lib/rustlib/x86_64-pc-windows-gnu/bin/self-contained;$env:PATH"
$env:CARGO_HOME = Join-Path $tools 'cargo-cache'
$env:CARGO_TARGET_DIR = Join-Path $tools 'rust-probe-target'
$env:RUSTFLAGS = '-C link-self-contained=yes'
```

Não misture esse target GNU com o wrapper GCC LLVM-MinGW. Em outra máquina, prepare Rust 1.99.0 GNU x64 e GNU binutils compatível; ajuste os diretórios da sessão. Não é necessário alterar PATH global, política de execução ou instalar a aplicação como administrador.

Compilar e testar:

```powershell
cargo test --locked
cargo build --release --locked
pnpm install --frozen-lockfile
pnpm check
pnpm test
pnpm test:rust
```

Cargo test cobre conversão de fusos; pnpm test mantém o teste do fallback Node; pnpm test:rust inicia a aplicação release e cobre o protocolo, contrato, erros, recuperação, identidade por execução, stderr e EOF. Node e pnpm são ferramentas de teste e operação, não dependências do executável Rust em execução. Use os binários já configurados nesta máquina se eles não estiverem no PATH do terminal externo.

pnpm start é um atalho de desenvolvimento para cargo run --release --locked; pnpm start:node executa o fallback. No túnel use o executável Rust diretamente, sem Cargo, pnpm ou compilação na inicialização.

Executar manualmente:

```powershell
& "$env:CARGO_TARGET_DIR/release/recallly-probe.exe"
```

Pare com Ctrl+C. O cliente MCP deve iniciar esse executável, sem argumentos. Pode definir RECALLLY_PROBE_EXE para testar um artefato em outro local; caso contrário o teste usa CARGO_TARGET_DIR/release/recallly-probe.exe ou target/release/recallly-probe.exe. Não envie texto livre ao stdin.

Para uma medição curta, defina RECALLLY_MEASUREMENT_FILE para um JSON externo ao checkout, RECALLLY_POWERSHELL para pwsh.exe e execute pnpm test:rust. São dez segundos de estabilização e 15 amostras a cada dois segundos, sem compilar em paralelo. O resultado contém apenas versão e métricas numéricas por função do processo, sem UUID, caminhos ou nomes da máquina.

## Túnel oficial e reversão

O plugin instalado não deve ser recriado. O perfil original Node e o cliente oficial Secure MCP Tunnel 0.0.16 ficam fora do checkout, em ../secure-mcp/profiles e ../tools/secure-mcp/v0.0.16 na estrutura preparada. O perfil usa uma referência à variável CONTROL_PLANE_API_KEY, nunca uma chave gravada. Não altere o identificador ou a associação existentes. O [guia oficial](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels) documenta conexão de saída HTTPS e stdio. Não publique portas nem use túnel de terceiros.

A configuração operacional local/tunnel-config.json já foi preparada nesta máquina e é ignorada pelo Git. Ela contém somente caminhos locais e o nome do perfil: node, nodeServer, rustExecutable, tunnel, powershell, profile, profileDir e healthFile. Não coloque credenciais nesse JSON. Em outro ambiente, configure esses campos para executáveis reais e perfil externo existente; healthFile deve ficar fora do checkout. O iniciador não cria nem sobrescreve o perfil.

Iniciar Rust no computador preparado, a partir da raiz do checkout:

```powershell
.\scripts\Connect-Probe.cmd rust
```

O iniciador pede a chave no campo protegido. Cole o valor completo usando o botão direito, sem aspas, e pressione Enter. Nunca envie a chave ao chat. A chave de runtime precisa estar válida e ter Tunnels Read + Use no workspace associado. Formato aprovado por doctor não comprova autenticação.

Antes de trocar o subprocesso, o iniciador valida a aplicação localmente e executa doctor. Só então encerra uma árvore de túnel que corresponda ao executável e ao perfil configurados; recusa substituir túnel não relacionado. Inicia o cliente oficial com --mcp.command apontando para Rust somente nessa execução, sem salvar a troca no perfil. Confere healthz e readyz exclusivamente em 127.0.0.1 e mantém a janela aberta. Mensagens de operação não imprimem credenciais, caminhos ou respostas com UUID.

Para parar, pressione Ctrl+C e aguarde o retorno ao prompt. O iniciador solicita encerramento do túnel e seus filhos. Se fechar a janela à força, confira os processos antes de iniciar novamente.

Restaurar Node imediatamente, usando o mesmo perfil e uma chave válida no campo protegido:

```powershell
.\scripts\Connect-Probe.cmd node
```

Esse comando valida src/server.mjs, encerra a árvore configurada e inicia o túnel com o comando Node preservado. Não apaga Rust nem altera o plugin. A operação requer uma chave válida; sem ela, preserve um daemon funcional já existente, não tente extrair seu ambiente. O segredo não é persistido pelo iniciador.

## Validação pelo chat

Após o túnel Rust ficar pronto, selecione o plugin Recallly Probe já instalado e envie: `Use Recallly Probe e chame recallly_status sem argumentos. Confirme a versão 0.2.0. Não use o terminal para simular a chamada.`

Uma resposta local ou readiness não comprova essa chamada. A prova comparativa Rust 0.1.0 respondeu pelo plugin anteriormente; isso não valida automaticamente a aplicação principal 0.2.0. A validação remota desta versão permanece pendente até uma chamada real. O computador, a rede, a chave e o daemon precisam continuar disponíveis; não há inicialização automática.
