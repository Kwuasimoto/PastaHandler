# Dev watch loop: any save under src/ or assets/ rebuilds and relaunches
# both processes. Ctrl+C to stop. Requires cargo-watch (cargo install cargo-watch).
Set-Location (Split-Path $PSScriptRoot -Parent)
cargo watch -w src -w assets -s "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\restart.ps1"
