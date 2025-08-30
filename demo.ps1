# PassMan CLI Demo Script (PowerShell)
# This script demonstrates the Rust CLI functionality

Write-Host "🛡️ PassMan CLI Demo" -ForegroundColor Blue
Write-Host "====================" -ForegroundColor Blue

Write-Host ""
Write-Host "📋 Available Commands:" -ForegroundColor Green
Write-Host "----------------------" -ForegroundColor Green

# Show help
Write-Host "1. Getting help:" -ForegroundColor Yellow
Write-Host "cargo run -- --help" -ForegroundColor Gray

Write-Host ""
Write-Host "2. Adding a new entry:" -ForegroundColor Yellow
Write-Host 'cargo run -- add "Gmail" "user@gmail.com" "SecurePassword123!" --verbose' -ForegroundColor Gray

Write-Host ""
Write-Host "3. Listing entries:" -ForegroundColor Yellow
Write-Host "cargo run -- list --detailed" -ForegroundColor Gray

Write-Host ""
Write-Host "4. Searching entries:" -ForegroundColor Yellow
Write-Host 'cargo run -- find "gmail"' -ForegroundColor Gray

Write-Host ""
Write-Host "5. Generating passwords:" -ForegroundColor Yellow
Write-Host "cargo run -- generate --length 16 --symbols --count 3" -ForegroundColor Gray

Write-Host ""
Write-Host "6. Checking password strength:" -ForegroundColor Yellow
Write-Host 'cargo run -- check-strength "mypassword123"' -ForegroundColor Gray

Write-Host ""
Write-Host "7. Viewing vault statistics:" -ForegroundColor Yellow
Write-Host "cargo run -- stats" -ForegroundColor Gray

Write-Host ""
Write-Host "8. Viewing audit logs:" -ForegroundColor Yellow
Write-Host "cargo run -- logs --count 5" -ForegroundColor Gray

Write-Host ""
Write-Host "9. Benchmarking crypto performance:" -ForegroundColor Yellow
Write-Host "cargo run -- benchmark" -ForegroundColor Gray

Write-Host ""
Write-Host "💡 Tips:" -ForegroundColor Cyan
Write-Host "--------" -ForegroundColor Cyan
Write-Host "• Use --verbose flag for detailed logging" -ForegroundColor White
Write-Host "• Master password is required for most operations" -ForegroundColor White
Write-Host "• Vault automatically locks after 15 minutes of inactivity" -ForegroundColor White
Write-Host "• All data is encrypted with XChaCha20Poly1305" -ForegroundColor White
Write-Host "• Use strong master passwords (12+ characters recommended)" -ForegroundColor White

Write-Host ""
Write-Host "🔗 Frontend Interface:" -ForegroundColor Cyan
Write-Host "---------------------" -ForegroundColor Cyan
Write-Host "• Web UI: http://localhost:3000" -ForegroundColor White
Write-Host "• Demo master password: any 8+ character string" -ForegroundColor White
Write-Host "• Features: vault management, password generation, search" -ForegroundColor White

Write-Host ""
Write-Host "🚀 Quick Start:" -ForegroundColor Magenta
Write-Host "--------------" -ForegroundColor Magenta
Write-Host "1. Build the project: cargo build --release" -ForegroundColor White
Write-Host "2. Start frontend: cd frontend && npm run dev" -ForegroundColor White
Write-Host "3. Open browser: http://localhost:3000" -ForegroundColor White
