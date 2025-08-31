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
