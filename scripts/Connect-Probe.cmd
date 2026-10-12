@echo off
setlocal
set "RECALLLY_ROOT=%~dp0.."
set "RECALLLY_MODE=%~1"
if "%RECALLLY_MODE%"=="" set "RECALLLY_MODE=rust"
powershell.exe -NoProfile -Command "$ErrorActionPreference='Stop'; $secret=Read-Host 'Cole a chave completa com o botao direito, sem aspas' -AsSecureString; try { $value=([System.Net.NetworkCredential]::new('', $secret).Password).Trim(); if($value -cnotmatch '\A[0-9A-Za-z_-]+\z'){Write-Output 'Formato invalido. Cole somente o valor completo com o botao direito.'; exit 2}; $env:CONTROL_PLANE_API_KEY=$value; $value=$null; $config=Get-Content -LiteralPath (Join-Path $env:RECALLLY_ROOT 'local/tunnel-config.json') -Raw | ConvertFrom-Json; & $config.node (Join-Path $env:RECALLLY_ROOT 'scripts/tunnel.mjs') $env:RECALLLY_MODE; exit $LASTEXITCODE } finally { Remove-Item Env:CONTROL_PLANE_API_KEY -ErrorAction SilentlyContinue; $value=$null; if($secret){$secret.Dispose()}; $secret=$null }"
exit /b %errorlevel%
