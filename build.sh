#!/bin/bash

# Build script for PassMann browser extension

echo "🔧 Building PassMann Browser Extension..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack not found. Install with: cargo install wasm-pack"
    exit 1
fi

# Build the WASM module
echo "🦀 Building Rust WASM module..."
cd wasm
wasm-pack build --target web --out-dir ../extension/wasm --no-typescript

if [ $? -ne 0 ]; then
    echo "❌ WASM build failed"
    exit 1
fi

cd ..

# Copy icons (create placeholder icons if they don't exist)
echo "🎨 Setting up icons..."
mkdir -p extension/icons

# Create simple SVG icons if they don't exist
if [ ! -f "extension/icons/icon16.png" ]; then
    echo "Creating placeholder icons..."
    # You can replace these with actual icon files
    for size in 16 32 48 128; do
        echo '<?xml version="1.0" encoding="UTF-8"?>
<svg width="'$size'" height="'$size'" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="24" height="24" rx="4" fill="#3B82F6"/>
<path d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>' > "extension/icons/icon$size.svg"
    done
fi

# Update popup.html to load WASM
echo "📦 Updating extension files..."

# Create a simple WASM loader
cat > extension/wasm-loader.js << 'EOF'
// WASM Loader for PassMann
let wasmModule = null;

async function initWasm() {
  try {
    const wasmPath = chrome.runtime.getURL('wasm/PassMann_wasm.js');
    const { default: init, PassMannWasm } = await import(wasmPath);
    await init();
    wasmModule = new PassMannWasm();
    console.log('✅ WASM module loaded successfully');
    return wasmModule;
  } catch (error) {
    console.error('❌ Failed to load WASM:', error);
    return null;
  }
}

// Export for use in popup
window.PassMannWasm = {
  init: initWasm,
  getInstance: () => wasmModule
};
EOF

echo "✅ Build complete!"
echo ""
echo "📁 Extension files are in: ./extension/"
echo "🚀 To install in Chrome:"
echo "   1. Open Chrome and go to chrome://extensions/"
echo "   2. Enable 'Developer mode'"
echo "   3. Click 'Load unpacked' and select the ./extension/ folder"
echo ""
echo "🔧 To rebuild: npm run build"
