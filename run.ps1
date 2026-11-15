$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
$localCargo = Join-Path $PSScriptRoot '.tools\cargo\bin\cargo.exe'
if (Test-Path -LiteralPath $localCargo) {
    $env:CARGO_HOME = Join-Path $PSScriptRoot '.tools\cargo'
    $env:RUSTUP_HOME = Join-Path $PSScriptRoot '.tools\rustup'
    & $localCargo run --locked
} elseif (Get-Command cargo -ErrorAction SilentlyContinue) {
    cargo run --locked
} else {
    Write-Error 'Brak kompilatora Rust. Zainstaluj Rust z https://rustup.rs i uruchom ponownie.'
}
exit $LASTEXITCODE
