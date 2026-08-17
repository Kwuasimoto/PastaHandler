# Kill running instances, rebuild, relaunch resident + settings window.
# Used by dev.ps1 on every file change; also fine to run by hand.
Get-Process pastahandler -ErrorAction SilentlyContinue | Stop-Process -Force
cargo build
if ($LASTEXITCODE -eq 0) {
    Start-Process "target\debug\pastahandler.exe"
    Start-Process "target\debug\pastahandler.exe" -ArgumentList "--settings"
    Write-Host "relaunched resident + settings" -ForegroundColor Green
} else {
    Write-Host "build failed - windows left closed" -ForegroundColor Red
}
