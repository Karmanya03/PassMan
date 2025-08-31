# Build script for PassMann browser extension (Windows)

Write-Host "Building PassMann Browser Extension..." -ForegroundColor Blue

# Check if wasm-pack is installed
if (!(Get-Command "wasm-pack" -ErrorAction SilentlyContinue)) {
    Write-Host "wasm-pack not found. Install with: cargo install wasm-pack" -ForegroundColor Red
    exit 1
}

# Build the WASM module
Write-Host "Building Rust WASM module..." -ForegroundColor Yellow
Set-Location wasm
& wasm-pack build --target web --out-dir ../extension/wasm --no-typescript

if ($LASTEXITCODE -ne 0) {
    Write-Host "WASM build failed" -ForegroundColor Red
    exit 1
}

Set-Location ..

# Create icons directory
Write-Host "Setting up icons..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path "extension/icons" | Out-Null

# Create placeholder SVG icons if they don't exist
$iconSizes = @(16, 32, 48, 128)
foreach ($size in $iconSizes) {
    $iconPath = "extension/icons/icon$size.svg"
    if (!(Test-Path $iconPath)) {
        $svgContent = @"
<?xml version="1.0" encoding="UTF-8"?>
<svg width="$size" height="$size" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="24" height="24" rx="4" fill="#3B82F6"/>
<path d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
"@
        Set-Content -Path $iconPath -Value $svgContent
    }
}

# Create WASM loader
Write-Host "Creating WASM loader..." -ForegroundColor Yellow
$wasmLoaderContent = @'
// WASM Loader for PassMann
let wasmModule = null;

async function initWasm() {
  try {
    const wasmPath = chrome.runtime.getURL('wasm/PassMann_wasm.js');
    const { default: init, PassMannWasm } = await import(wasmPath);
    await init();
    wasmModule = new PassMannWasm();
    console.log('WASM module loaded successfully');
    return wasmModule;
  } catch (error) {
    console.error('Failed to load WASM:', error);
    return null;
  }
}

// Export for use in popup
window.PassMannWasm = {
  init: initWasm,
  getInstance: () => wasmModule
};
'@

Set-Content -Path "extension/wasm-loader.js" -Value $wasmLoaderContent

Write-Host "Build complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Extension files are in: ./extension/" -ForegroundColor Cyan
Write-Host "To install in Chrome:" -ForegroundColor Cyan
Write-Host "   1. Open Chrome and go to chrome://extensions/" -ForegroundColor White
Write-Host "   2. Enable 'Developer mode'" -ForegroundColor White
Write-Host "   3. Click 'Load unpacked' and select the ./extension/ folder" -ForegroundColor White
Write-Host ""
Write-Host "To rebuild: .\build.ps1" -ForegroundColor Cyan
