class PassMannApp {
  constructor() {
    this.wasm = null;
    this.wasmType = null; // Track whether we're using 'rust' or 'js' implementation
    this.storageManager = new SQLiteLikeStorageManager(this);
    this.state = {
      isLocked: true,
      entries: [],
      searchTerm: '',
      showAddForm: false,
      showLoginForm: true,
      showRegisterForm: false,
      showSettings: false,
      showStorageSettings: false,
      masterPassword: '',
      error: null,
      success: null,
      loading: true,
      user: null,
      cloudSyncEnabled: false,
      lastSync: null,
      storageMode: 'local', // 'local' or 'cloud'
      failedAttempts: 0
    };
    this.init();
  }

  async init() {
    try {
      // Try to load WASM module first
      try {
        // Try different WASM file names in order of preference
        const wasmFiles = ['passmann_wasm.js', 'passmnn_wasm.js', 'passman_wasm.js'];
        let wasmModule = null;
        let wasmError = null;
        
        for (const wasmFile of wasmFiles) {
          try {
            console.log(`Attempting to load WASM file: ${wasmFile}`);
            wasmModule = await import(chrome.runtime.getURL(`wasm/${wasmFile}`));
            
            // Get corresponding .wasm file
            const wasmBinaryFile = wasmFile.replace('.js', '_bg.wasm');
            const wasmBinary = await fetch(chrome.runtime.getURL(`wasm/${wasmBinaryFile}`));
            const wasmArrayBuffer = await wasmBinary.arrayBuffer();
            
            await wasmModule.default({ module_or_path: wasmArrayBuffer });
            console.log(`✓ Successfully loaded WASM: ${wasmFile}`);
            break;
          } catch (e) {
            console.warn(`Failed to load ${wasmFile}:`, e);
            wasmError = e;
            continue;
          }
        }
        
        if (!wasmModule) {
          throw new Error(`All WASM files failed to load. Last error: ${wasmError?.message}`);
        }
        
        this.wasm = new wasmModule.PassMannWasm();
        this.wasmType = 'rust';
        console.log('WASM loaded successfully');
        
        // Debug: Check available methods
        console.log('Available WASM module methods:', Object.getOwnPropertyNames(wasmModule));
        console.log('WASM instance methods:', Object.getOwnPropertyNames(this.wasm));
        
        // Test if critical methods exist
        const criticalMethods = ['add_entry', 'get_entries_json', 'encrypt_vault', 'unlock_vault'];
        criticalMethods.forEach(method => {
          if (typeof this.wasm[method] === 'function') {
            console.log(`✓ ${method} is available`);
          } else {
            console.error(`✗ ${method} is NOT available`);
          }
        });
      } catch (wasmError) {
        console.warn('WASM failed to load, using JavaScript fallback:', wasmError);
        // Use JavaScript crypto fallback
        this.wasm = new window.CryptoJS();
        this.wasmType = 'js';
        console.log('JavaScript crypto fallback loaded');
      }
      
      this.state.loading = false;
    } catch (error) {
      console.error('Failed to initialize crypto:', error);
      this.state.error = 'Failed to load encryption module: ' + error.message;
      this.state.loading = false;
    }
    
    // Check for existing cloud session
    await this.checkCloudSession();
    
    await this.loadState();
    
    // Check session validity and auto-lock if needed
    const sessionValid = await this.checkSessionValidity();
    if (sessionValid && this.state.isLocked) {
      // We have a valid session but UI shows locked - this is normal after initialization
      console.log('Valid session detected but vault is locked - user will need to unlock to restore state');
    } else if (!sessionValid) {
      // No valid session, ensure we're locked
      this.state.isLocked = true;
      console.log('No valid session found, vault will remain locked');
    } else if (sessionValid && !this.state.isLocked) {
      // Session is valid and we're already unlocked - good state
      console.log('Valid session and vault is unlocked - restored session successfully');
    }
    
    this.render();
    this.bindEvents();
    this.initializeFoxMascot();
    this.checkPendingSaves(); // Check for passwords to save from content script
    
    // Set up periodic session checks
    this.startSessionMonitoring();
    
    // Cleanup on page unload
    window.addEventListener('beforeunload', () => {
      this.stopSessionMonitoring();
    });
  }

  async loadState() {
    try {
      // Load storage mode preference and failed attempts
      const result = await chrome.storage.local.get(['passmann_storage_mode', 'passmann_settings', 'passmann_failed_attempts']);
      this.state.storageMode = result.passmann_storage_mode || 'local';
      this.state.failedAttempts = result.passmann_failed_attempts || 0;
      
      // Set WASM storage mode if available
      if (this.wasm && this.wasmType === 'rust') {
        this.wasm.set_storage_mode(this.state.storageMode);
      }
      
      // Load vault data using enhanced storage manager
      const vaultData = await this.storageManager.loadVault();
      if (vaultData) {
        this.state.lastSync = vaultData.timestamp;
        console.log(`Vault data loaded from ${vaultData.storageMode} storage`);
      }
      
      // Check session validity before setting lock state
      const sessionValid = await this.checkSessionValidity();
      if (sessionValid) {
        // Check if we have cached master password or session data
        const sessionData = await chrome.storage.session.get(['passmann_session_key', 'passmann_master_password_hash', 'passmann_session_start']);
        if (sessionData.passmann_session_key && sessionData.passmann_session_start) {
          const sessionAge = Date.now() - sessionData.passmann_session_start;
          const maxSessionTime = 60 * 60 * 1000; // 60 minutes
          
          if (sessionAge < maxSessionTime) {
            console.log(`Valid session found (${Math.round((maxSessionTime - sessionAge) / 1000 / 60)} minutes remaining), restoring unlocked state`);
            this.state.isLocked = false; // Keep unlocked if session is valid
            
            // Try to restore WASM state if we have session data
            if (this.wasm && this.wasmType === 'rust' && sessionData.passmann_master_password_hash) {
              try {
                const salt = await this.getSalt();
                if (salt && vaultData && vaultData.encryptedData) {
                  // Attempt to unlock WASM with existing vault data
                  const unlocked = this.wasm.unlock_vault('', salt, Array.from(vaultData.encryptedData));
                  if (unlocked) {
                    console.log('WASM vault restored from session');
                    await this.loadVaultEntries();
                  } else {
                    console.log('Failed to restore WASM vault automatically, keeping UI unlocked but WASM may need password');
                    // Keep UI unlocked since session is valid, user won't need to re-enter password
                  }
                } else {
                  console.log('No vault data to restore, but session is valid');
                }
              } catch (error) {
                console.warn('Failed to restore WASM state:', error);
                // Don't lock the UI, session is still valid
              }
            }
            
            // Restart session monitoring since popup was reopened
            this.startSessionMonitoring();
          } else {
            console.log('Session expired, locking vault');
            this.state.isLocked = true;
            await this.clearSessionData();
          }
        } else {
          console.log('Session valid but no session key found, locking vault');
          this.state.isLocked = true;
        }
      } else {
        console.log('No valid session found, locking vault');
        this.state.isLocked = true;
      }
      
    } catch (error) {
      console.error('Failed to load state:', error);
      this.state.error = 'Failed to load application state';
      this.state.isLocked = true; // Default to locked on error
    }
  }

  async hashPassword(password) {
    try {
      const encoder = new TextEncoder();
      const data = encoder.encode(password);
      const hashBuffer = await crypto.subtle.digest('SHA-256', data);
      const hashArray = Array.from(new Uint8Array(hashBuffer));
      return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
    } catch (error) {
      console.error('Failed to hash password:', error);
      return null;
    }
  }

  async clearSessionData() {
    try {
      await chrome.storage.session.clear();
      console.log('Session data cleared');
    } catch (error) {
      console.error('Failed to clear session data:', error);
    }
  }

  async saveState() {
    try {
      console.log('saveState called, current state:', {
        isLocked: this.state.isLocked,
        hasWasm: !!this.wasm,
        wasmType: this.wasmType,
        entriesCount: this.state.entries.length
      });
      
      // Save storage mode preference
      await chrome.storage.local.set({
        passmann_storage_mode: this.state.storageMode
      });
      
      // Save vault data if we have entries and are unlocked
      if (!this.state.isLocked && this.wasm && this.wasmType === 'rust') {
        console.log('Attempting to encrypt vault...');
        
        // Check if WASM vault is properly initialized
        let wasmEntriesCount = 0;
        if (this.wasm.get_entries_count) {
          wasmEntriesCount = this.wasm.get_entries_count();
          console.log(`WASM vault has ${wasmEntriesCount} entries, state has ${this.state.entries.length} entries`);
        }
        
        // Only sync entries if WASM vault is empty but we have entries in state
        if (this.state.entries.length > 0 && wasmEntriesCount === 0) {
          console.log('WASM vault is empty, syncing entries from state...');
          
          // Add all current entries to WASM vault
          for (const entry of this.state.entries) {
            try {
              const result = this.wasm.add_entry(
                entry.service || '', 
                entry.username || '', 
                entry.password || '', 
                entry.url || null, 
                entry.notes || null
              );
              console.log(`Added entry ${entry.service} to WASM vault: ${result}`);
            } catch (addError) {
              console.error(`Failed to add entry ${entry.service} to WASM vault:`, addError);
            }
          }
          
          // Verify entries were added
          if (this.wasm.get_entries_count) {
            const newWasmCount = this.wasm.get_entries_count();
            console.log(`WASM vault now has ${newWasmCount} entries after sync`);
          }
        } else if (wasmEntriesCount > 0) {
          console.log('WASM vault already has entries, skipping sync');
        } else {
          console.log('No entries to sync to WASM vault');
        }
        
        // Check if WASM is properly unlocked before encryption
        let isWasmUnlocked = false;
        if (this.wasm.is_unlocked) {
          isWasmUnlocked = this.wasm.is_unlocked();
          console.log('WASM unlocked status:', isWasmUnlocked);
        }
        
        if (!isWasmUnlocked) {
          console.error('WASM is not properly unlocked, cannot encrypt vault');
          await this.saveVaultFallback();
          return;
        }
        
        // Try to encrypt the vault
        let encryptedVault = null;
        try {
          // Debug WASM state before encryption
          console.log('=== Pre-encryption WASM state ===');
          this.debugWasmState();
          
          encryptedVault = this.wasm.encrypt_vault();
          console.log('encrypt_vault result:', encryptedVault ? `Success (${encryptedVault.length} bytes)` : 'Failed/Empty');
        } catch (encryptError) {
          console.error('WASM encrypt_vault threw error:', encryptError);
        }
        
        if (encryptedVault && encryptedVault.length > 0) {
          const salt = await this.getSalt();
          if (salt) {
            console.log('Saving encrypted vault to storage...');
            await this.storageManager.saveVault(new Uint8Array(encryptedVault), salt);
            this.state.lastSync = Date.now();
            console.log('Vault data saved successfully');
            
            // Update session activity
            await chrome.storage.session.set({
              passmann_last_activity: Date.now()
            });
          } else {
            console.error('No salt available for saving vault');
          }
        } else {
          console.warn('encrypt_vault returned empty result, using fallback encryption');
          // Enhanced fallback encryption
          await this.saveVaultFallback();
        }
      } else {
        console.log('Skipping vault save:', {
          isLocked: this.state.isLocked,
          hasWasm: !!this.wasm,
          wasmType: this.wasmType
        });
      }
    } catch (error) {
      console.error('Failed to save state:', error);
      this.state.error = 'Failed to save application state';
    }
  }

  debugWasmState() {
    if (!this.wasm || this.wasmType !== 'rust') {
      console.log('WASM Debug: Not using Rust WASM or WASM not loaded');
      return;
    }
    
    console.log('=== WASM Vault Debug State ===');
    
    try {
      if (this.wasm.is_unlocked) {
        const unlocked = this.wasm.is_unlocked();
        console.log(`WASM Unlocked: ${unlocked}`);
      }
      
      if (this.wasm.get_entries_count) {
        const count = this.wasm.get_entries_count();
        console.log(`WASM Entries Count: ${count}`);
      }
      
      if (this.wasm.get_entries_json) {
        const entriesJson = this.wasm.get_entries_json();
        if (entriesJson) {
          const entries = JSON.parse(entriesJson);
          console.log(`WASM Entries: ${entries.length} entries`);
          entries.forEach((entry, index) => {
            console.log(`  Entry ${index}: ${entry.service} (${entry.username})`);
          });
        } else {
          console.log('WASM Entries: No entries or null response');
        }
      }
      
      if (this.wasm.get_storage_mode) {
        const mode = this.wasm.get_storage_mode();
        console.log(`WASM Storage Mode: ${mode}`);
      }
      
    } catch (error) {
      console.error('Error debugging WASM state:', error);
    }
    
    console.log('=== End WASM Debug ===');
  }

  async saveVaultFallback() {
    try {
      if (this.state.entries.length > 0) {
        console.log('Saving entries using fallback encryption...');
        const entriesJson = JSON.stringify(this.state.entries);
        let encryptedFallback = null;
        
        // Try WASM encrypt_data first
        if (this.wasm && this.wasm.encrypt_data) {
          try {
            encryptedFallback = this.wasm.encrypt_data(entriesJson);
            if (encryptedFallback) {
              console.log('Used WASM encrypt_data for fallback');
            }
          } catch (wasmError) {
            console.warn('WASM encrypt_data failed:', wasmError);
          }
        }
        
        // Use CryptoJS fallback if WASM failed
        if (!encryptedFallback && window.CryptoJS && this.state.masterPassword) {
          try {
            const salt = CryptoJS.lib.WordArray.random(16);
            const key = CryptoJS.PBKDF2(this.state.masterPassword, salt, {
              keySize: 256/32,
              iterations: 100000
            });
            const encrypted = CryptoJS.AES.encrypt(entriesJson, key, {
              iv: salt
            });
            encryptedFallback = {
              data: encrypted.toString(),
              salt: salt.toString()
            };
            console.log('Used CryptoJS for fallback encryption');
          } catch (cryptoError) {
            console.error('CryptoJS fallback failed:', cryptoError);
          }
        }
        
        if (encryptedFallback) {
          const salt = await this.getSalt();
          if (salt) {
            await this.storageManager.saveVault(
              typeof encryptedFallback === 'string' 
                ? new TextEncoder().encode(encryptedFallback)
                : new TextEncoder().encode(JSON.stringify(encryptedFallback)), 
              salt
            );
            console.log('Entries saved using fallback encryption');
            this.state.lastSync = Date.now();
            
            // Update session activity
            await chrome.storage.session.set({
              passmann_last_activity: Date.now()
            });
          }
        } else {
          console.error('All encryption methods failed');
        }
      }
    } catch (error) {
      console.error('Fallback save failed:', error);
    }
  }

  async loadVaultEntries() {
    try {
      if (!this.wasm || this.state.isLocked) {
        return;
      }
      
      // Get all entries from the WASM vault
      if (this.wasmType === 'rust' && this.wasm.get_entries_json) {
        try {
          const entriesJson = this.wasm.get_entries_json();
          if (entriesJson) {
            const entries = JSON.parse(entriesJson);
            if (Array.isArray(entries)) {
              this.state.entries = entries;
              console.log(`Loaded ${entries.length} entries from WASM vault`);
            }
          } else {
            console.log('No entries found in WASM vault, starting with empty vault');
            this.state.entries = [];
          }
        } catch (error) {
          console.error('Failed to get entries from WASM:', error);
          this.state.entries = [];
        }
      }
    } catch (error) {
      console.error('Failed to load vault entries:', error);
      this.state.entries = [];
    }
  }

  async getSalt() {
    try {
      const result = await chrome.storage.local.get(['passmann_salt']);
      return result.passmann_salt ? new Uint8Array(result.passmann_salt) : null;
    } catch (error) {
      console.error('Failed to get salt:', error);
      return null;
    }
  }

  async saveSalt(salt) {
    try {
      await chrome.storage.local.set({
        passmann_salt: Array.from(salt)
      });
    } catch (error) {
      console.error('Failed to save salt:', error);
    }
  }

  // Secure master password hashing
  async createMasterPasswordHash(password, salt) {
    const encoder = new TextEncoder();
    const passwordBytes = encoder.encode(password);
    
    // Create a combined buffer of password + salt
    const combined = new Uint8Array(passwordBytes.length + salt.length);
    combined.set(passwordBytes, 0);
    combined.set(salt, passwordBytes.length);
    
    // Use PBKDF2 with SHA-256 for secure hashing
    const keyMaterial = await crypto.subtle.importKey(
      'raw',
      passwordBytes,
      { name: 'PBKDF2' },
      false,
      ['deriveBits']
    );
    
    const hashBuffer = await crypto.subtle.deriveBits(
      {
        name: 'PBKDF2',
        salt: salt,
        iterations: 100000, // Strong iteration count
        hash: 'SHA-256'
      },
      keyMaterial,
      256 // 32 bytes
    );
    
    return new Uint8Array(hashBuffer);
  }

  // Secure constant-time comparison to prevent timing attacks
  secureCompare(a, b) {
    if (a.length !== b.length) {
      return false;
    }
    
    let result = 0;
    for (let i = 0; i < a.length; i++) {
      result |= a[i] ^ b[i];
    }
    
    return result === 0;
  }

  // Check session validity and auto-lock
  async checkSessionValidity() {
    try {
      // Get user-configurable session timeout (default 60 minutes)
      const settings = await chrome.storage.local.get(['passmann_session_timeout']);
      const sessionTimeoutMinutes = settings.passmann_session_timeout || 60;
      const maxSessionTime = sessionTimeoutMinutes * 60 * 1000;
      
      const sessionData = await chrome.storage.local.get(['passmann_session_start']);
      
      if (!sessionData.passmann_session_start) {
        // No session, need to lock the vault
        if (!this.state.isLocked) {
          this.lockVault();
        }
        return false;
      }
      
      const sessionAge = Date.now() - sessionData.passmann_session_start;
      
      if (sessionAge > maxSessionTime) {
        // Session expired, need to lock the vault
        if (!this.state.isLocked) {
          console.log(`Session expired after ${sessionTimeoutMinutes} minutes, locking vault`);
          this.lockVault();
        }
        return false;
      }
      
      // Session is still valid
      if (this.state.isLocked) {
        // Session valid but UI is locked - this can happen after extension restart
        // Try to restore session if WASM is already unlocked or master password is cached
        console.log('Valid session found, attempting to restore unlocked state');
        return true; // Let the caller handle restoration
      }
      
      return true;
    } catch (error) {
      console.error('Session check failed:', error);
      if (!this.state.isLocked) {
        this.lockVault();
      }
      return false;
    }
  }

  // Start periodic session monitoring
  startSessionMonitoring() {
    // Clear any existing interval
    if (this.sessionMonitorInterval) {
      clearInterval(this.sessionMonitorInterval);
    }
    
    // Check session validity every 5 minutes (less frequent checks)
    this.sessionMonitorInterval = setInterval(async () => {
      if (!this.state.isLocked) {
        const sessionValid = await this.checkSessionValidity();
        if (!sessionValid) {
          console.log('Session expired, locking vault');
          this.render(); // Re-render to show lock screen
        }
      }
    }, 5 * 60 * 1000); // Check every 5 minutes instead of every minute
    
    console.log('Session monitoring started (checking every 5 minutes)');
  }

  // Extend session on user activity
  async extendSession() {
    try {
      if (!this.state.isLocked) {
        await chrome.storage.local.set({ 
          passmann_session_start: Date.now()
        });
        console.log('Session extended due to user activity');
      }
    } catch (error) {
      console.error('Failed to extend session:', error);
    }
  }

  // Stop session monitoring (cleanup)
  stopSessionMonitoring() {
    if (this.sessionMonitorInterval) {
      clearInterval(this.sessionMonitorInterval);
      this.sessionMonitorInterval = null;
    }
  }

  // Lock the vault
  lockVault() {
    this.state.isLocked = true;
    this.state.masterPassword = '';
    // Don't clear entries here - only clear session data
    // The entries will be cleared only when session actually expires
    chrome.storage.local.remove(['passmann_session_start']);
    this.stopSessionMonitoring(); // Stop monitoring when locked
    this.render();
  }

  render() {
    try {
      const app = document.getElementById('app');
      const cubeAnimation = document.getElementById('cube-animation');
      
      // Show/hide cube animation based on lock state
      if (cubeAnimation) {
        if (this.state.isLocked && !this.state.loading && !this.state.error) {
          cubeAnimation.style.display = 'flex';
          document.body.classList.add('locked');
        } else {
          cubeAnimation.style.display = 'none';
          document.body.classList.remove('locked');
        }
      }
      
      if (this.state.loading) {
        app.innerHTML = this.renderLoading();
        return;
      }
      
      if (this.state.error) {
        app.innerHTML = `<div class="error">${this.state.error}</div>`;
        setTimeout(() => {
          this.state.error = null;
          this.render();
        }, 3000);
        return;
      }

      if (this.state.success) {
        app.innerHTML = `<div class="success">${this.state.success}</div>`;
        setTimeout(() => {
          this.state.success = null;
          this.render();
        }, 2000);
        return;
      }
      
      // Show login form if user is not authenticated with cloud
      if (this.state.showLoginForm && !this.state.user) {
        app.innerHTML = this.renderLoginScreen();
        return;
      }
      
      // Show register form
      if (this.state.showRegisterForm) {
        app.innerHTML = this.renderRegisterScreen();
        return;
      }
      
      if (this.state.isLocked) {
        app.innerHTML = this.renderLockScreen();
      } else {
        app.innerHTML = this.renderMainScreen();
      }
    } catch (error) {
      console.error('Render error:', error);
      const app = document.getElementById('app');
      app.innerHTML = `
        <div class="error">
          Interface error occurred. 
          <button onclick="window.passmannApp.state.error = null; window.passmannApp.render();">
            Retry
          </button>
        </div>
      `;
    }
  }

  createAnimatedButton(text, classes = 'btn btn-primary', onclick = '', id = '', dataAttributes = '') {
    const buttonId = id ? `id="${id}"` : '';
    const onclickAttr = onclick ? `onclick="${onclick}"` : '';
    const dataAttrs = dataAttributes || '';
    
    return `
      <div class="${classes}">
        <div class="button-container">
          <button class="real-button" ${buttonId} ${onclickAttr} ${dataAttrs}></button>
          <div class="backdrop"></div>
          <div class="spin spin-blur"></div>
          <div class="spin spin-intense"></div>
          <div class="spin spin-inside"></div>
          <div class="button-border">
            <div class="button">${text}</div>
          </div>
        </div>
      </div>
    `;
  }

  renderLoading() {
    return `
      <div class="loading">
        <div class="loading-spinner"></div>
        <div>Loading PassMann...</div>
      </div>
    `;
  }

  renderLockScreen() {
    // Check if user has failed attempts to show reset option
    const showResetOption = this.state.failedAttempts >= 3;
    
    return `
      <div class="glass">
        <div class="form-title">Unlock Vault</div>
        <div class="input-group">
          <input type="password" id="master-password" placeholder="Enter your master password" />
        </div>
        ${this.createAnimatedButton('🔓 Unlock', 'btn btn-primary', '', 'unlock-btn')}
        ${showResetOption ? `
          <div class="reset-section" style="margin-top: 20px; text-align: center;">
            <p style="color: #e74c3c; font-size: 12px; margin-bottom: 10px;">
              Forgot your master password?
            </p>
            ${this.createAnimatedButton('🔄 Reset Vault', 'btn btn-danger btn-small', '', 'reset-vault-btn')}
            <p style="color: #7f8c8d; font-size: 10px; margin-top: 5px;">
              Warning: This will delete all stored passwords
            </p>
          </div>
        ` : ''}
      </div>
      <div class="status">
        Enter your master password to access your secure vault
        ${showResetOption ? '<br><span style="color: #e74c3c;">Multiple failed attempts detected</span>' : ''}
      </div>
    `;
  }

  renderMainScreen() {
    if (this.state.showStorageSettings) {
      return this.renderStorageSettings();
    }
    
    if (this.state.showAddForm) {
      return this.renderAddForm();
    }

    const filteredEntries = this.state.entries.filter(entry => {
      // Safety checks to prevent undefined errors
      if (!entry) return false;
      
      const service = entry.service || entry.site || '';
      const username = entry.username || '';
      const searchTerm = this.state.searchTerm.toLowerCase();
      
      return service.toLowerCase().includes(searchTerm) ||
             username.toLowerCase().includes(searchTerm);
    });

    return `
      <div class="grid"></div>
      <div id="poda">
        <div class="glow"></div>
        <div class="darkBorderBg"></div>
        <div class="darkBorderBg"></div>
        <div class="darkBorderBg"></div>

        <div class="white"></div>

        <div class="border"></div>

        <div id="main">
          <input placeholder="Search..." type="text" id="search-input" class="input" value="${this.state.searchTerm}" />
          <div id="input-mask"></div>
          <div id="pink-mask"></div>
          <div id="search-icon">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="24"
              viewBox="0 0 24 24"
              stroke-width="2"
              stroke-linejoin="round"
              stroke-linecap="round"
              height="24"
              fill="none"
              class="feather feather-search"
            >
              <circle stroke="url(#search)" r="8" cy="11" cx="11"></circle>
              <line
                stroke="url(#searchl)"
                y2="16.65"
                y1="22"
                x2="16.65"
                x1="22"
              ></line>
              <defs>
                <linearGradient gradientTransform="rotate(50)" id="search">
                  <stop stop-color="#f8e7f8" offset="0%"></stop>
                  <stop stop-color="#b6a9b7" offset="50%"></stop>
                </linearGradient>
                <linearGradient id="searchl">
                  <stop stop-color="#b6a9b7" offset="0%"></stop>
                  <stop stop-color="#837484" offset="50%"></stop>
                </linearGradient>
              </defs>
            </svg>
          </div>
        </div>
      </div>
      
      <div class="entries-container">
        ${filteredEntries.length === 0 ? 
          '<div class="status">No password entries found<br>Click "Add Entry" to get started</div>' :
          filteredEntries.map(entry => this.renderEntry(entry)).join('')
        }
      </div>
      
      ${this.createAnimatedButton('➕ Add Entry', 'btn btn-success', '', 'add-entry-btn')}
      ${this.createAnimatedButton('⚙️ Storage Settings', 'btn btn-secondary', '', 'storage-settings-btn')}
      ${this.createAnimatedButton('🔒 Lock Vault', 'btn btn-danger', '', 'lock-btn')}
    `;
  }

  renderEntry(entry) {
    // Handle both 'service' and 'site' fields for compatibility
    const serviceName = entry.service || entry.site || 'Unknown Service';
    const username = entry.username || 'Unknown User';
    
    return `
      <div class="entry">
        <div class="entry-service">${this.escapeHtml(serviceName)}</div>
        <div class="entry-username">${this.escapeHtml(username)}</div>
        <div class="entry-actions">
          ${this.createAnimatedButton('🔗 Fill', 'btn-small', '', '', 'data-action="fill" data-entry-id="' + entry.id + '"')}
          ${this.createAnimatedButton('📋 Copy', 'btn-small', '', '', 'data-action="copy" data-entry-id="' + entry.id + '"')}
          ${this.createAnimatedButton('🗑️ Delete', 'btn-small btn-danger', '', '', 'data-action="delete" data-entry-id="' + entry.id + '"')}
        </div>
      </div>
    `;
  }

  renderAddForm() {
    return `
      <div class="form-container">
        <div class="form-title">Add New Entry</div>
        <div class="input-group">
          <label for="service-input">Service</label>
          <input type="text" id="service-input" placeholder="e.g., Gmail, Facebook" />
        </div>
        <div class="input-group">
          <label for="username-input">Username/Email</label>
          <input type="text" id="username-input" placeholder="your@email.com" />
        </div>
        <div class="input-group">
          <label for="password-input">Password</label>
          <input type="password" id="password-input" placeholder="Enter password" />
        </div>
        ${this.createAnimatedButton('🎲 Generate', 'btn btn-secondary', '', 'generate-password-btn')}
        ${this.createAnimatedButton('💾 Save Entry', 'btn btn-success', '', 'save-entry-btn')}
        ${this.createAnimatedButton('❌ Cancel', 'btn btn-danger', '', 'cancel-add-btn')}
      </div>
    `;
  }

  renderStorageSettings() {
    return `
      <div class="storage-settings-container">
        <div class="form-title">🏪 Storage Settings</div>
        
        <div class="storage-mode-selector">
          <div class="storage-option ${this.state.storageMode === 'local' ? 'active' : ''}" data-mode="local">
            <div class="storage-icon">💾</div>
            <div class="storage-info">
              <h4>Local Storage</h4>
              <p>Store passwords locally in browser with military-grade encryption</p>
              <div class="storage-features">
                ✅ Offline access<br>
                ✅ Maximum privacy<br>
                ❌ No sync between devices
              </div>
            </div>
          </div>
          
          <div class="storage-option ${this.state.storageMode === 'cloud' ? 'active' : ''}" data-mode="cloud">
            <div class="storage-icon">☁️</div>
            <div class="storage-info">
              <h4>Cloud Storage</h4>
              <p>Sync passwords across devices with zero-knowledge encryption</p>
              <div class="storage-features">
                ✅ Multi-device sync<br>
                ✅ Automatic backup<br>
                ✅ Zero-knowledge security
              </div>
            </div>
          </div>
        </div>
        
        ${this.state.storageMode === 'cloud' ? this.renderCloudConfig() : ''}
        
        <div class="storage-status">
          <h4>📊 Storage Status</h4>
          <div class="status-item">
            <span>Current Mode:</span> <strong>${this.state.storageMode.toUpperCase()}</strong>
          </div>
          <div class="status-item">
            <span>Last Sync:</span> <strong>${this.state.lastSync ? new Date(this.state.lastSync).toLocaleString() : 'Never'}</strong>
          </div>
          <div class="status-item">
            <span>Entries Count:</span> <strong>${this.state.entries.length}</strong>
          </div>
          ${this.state.storageMode === 'cloud' ? `
          <div class="status-item" id="cloud-verification">
            <span>Cloud Status:</span> <strong id="cloud-status-text">Click "Verify Cloud" to check</strong>
          </div>` : ''}
        </div>
        
        <div class="settings-actions">
          ${this.createAnimatedButton('💾 Save Settings', 'btn btn-primary', '', 'save-storage-settings-btn')}
          ${this.createAnimatedButton('🧪 Test Storage', 'btn btn-secondary', '', 'test-storage-btn')}
          ${this.state.storageMode === 'cloud' ? this.createAnimatedButton('☁️ Verify Cloud', 'btn btn-secondary', '', 'verify-cloud-btn') : ''}
          ${this.createAnimatedButton('🔄 Force Sync', 'btn btn-secondary', '', 'force-sync-btn')}
          ${this.createAnimatedButton('❌ Cancel', 'btn btn-danger', '', 'cancel-storage-settings-btn')}
        </div>
      </div>
    `;
  }

  renderCloudConfig() {
    return `
      <div class="cloud-config">
        <h4>☁️ Cloud Configuration</h4>
        <div class="input-group">
          <label for="cloud-email">Email:</label>
          <input type="email" id="cloud-email" placeholder="your@email.com" 
                 value="${this.state.user?.email || ''}" />
        </div>
        <div class="input-group">
          <label for="cloud-server">Server URL:</label>
          <input type="url" id="cloud-server" placeholder="https://your-passmnn-server.com" 
                 value="https://mzrgemojflreupcvrhww.supabase.co" />
        </div>
        <div class="sync-status">
          <strong>Connection Status:</strong> 
          <span class="${this.state.cloudSyncEnabled ? 'connected' : 'disconnected'}">
            ${this.state.cloudSyncEnabled ? '✅ Connected' : '❌ Not Connected'}
          </span>
        </div>
      </div>
    `;
  }

  renderLoginScreen() {
    return `
      <div class="auth-container">
        <div class="auth-header">
          <h1>Welcome to PassMann</h1>
          <p>Secure Password Manager with Cloud Sync</p>
        </div>
        
        <div class="glass auth-form">
          <div class="form-title">Login to Your Account</div>
          
          <div class="input-group">
            <label for="login-email">Email</label>
            <input type="email" id="login-email" placeholder="your@email.com" />
          </div>
          
          <div class="input-group">
            <label for="login-password">Password</label>
            <input type="password" id="login-password" placeholder="Enter your password" />
          </div>
          
          ${this.createAnimatedButton('🔐 Login', 'btn btn-primary', '', 'login-btn')}
          
          <div class="auth-footer">
            <p>Don't have an account? 
              <a href="#" id="show-register">Create one</a>
            </p>
            <p>
              <a href="#" id="skip-login">Skip and use local only</a>
            </p>
          </div>
        </div>
        
        <div class="features">
          <div class="feature">
            <span class="feature-icon">🔒</span>
            <span>Military-grade encryption</span>
          </div>
          <div class="feature">
            <span class="feature-icon">☁️</span>
            <span>Secure cloud sync</span>
          </div>
          <div class="feature">
            <span class="feature-icon">🌐</span>
            <span>Access anywhere</span>
          </div>
        </div>
      </div>
    `;
  }

  renderRegisterScreen() {
    return `
      <div class="auth-container">
        <div class="auth-header">
          <h1>Create Your Account</h1>
          <p>Join PassMann for secure password management</p>
        </div>
        
        <div class="glass auth-form">
          <div class="form-title">Create Account</div>
          
          <div class="input-group">
            <label for="register-email">Email</label>
            <input type="email" id="register-email" placeholder="your@email.com" />
          </div>
          
          <div class="input-group">
            <label for="register-password">Password</label>
            <input type="password" id="register-password" placeholder="Min. 12 characters" />
          </div>
          
          <div class="input-group">
            <label for="register-confirm">Confirm Password</label>
            <input type="password" id="register-confirm" placeholder="Confirm your password" />
          </div>
          
          ${this.createAnimatedButton('✨ Create Account', 'btn btn-primary', '', 'register-btn')}
          
          <div class="auth-footer">
            <p>Already have an account? 
              <a href="#" id="show-login">Login here</a>
            </p>
          </div>
        </div>
        
        <div class="security-info">
          <h3>🛡️ Your Security Matters</h3>
          <ul>
            <li>✅ End-to-end encryption</li>
            <li>✅ Zero-knowledge architecture</li>
            <li>✅ Local + cloud backup</li>
            <li>✅ Auto-lock protection</li>
          </ul>
        </div>
      </div>
    `;
  }

  bindEvents() {
    const app = document.getElementById('app');
    
    app.addEventListener('click', (e) => {
      // Extend session on any user interaction (if unlocked)
      this.extendSession();
      
      const target = e.target;
      const action = target.dataset.action;
      const entryId = target.dataset.entryId;

      if (target.id === 'unlock-btn') {
        this.handleUnlock();
      } else if (target.id === 'lock-btn') {
        this.handleLock();
      } else if (target.id === 'login-btn') {
        this.handleLogin();
      } else if (target.id === 'register-btn') {
        this.handleRegister();
      } else if (target.id === 'show-register') {
        e.preventDefault();
        this.state.showLoginForm = false;
        this.state.showRegisterForm = true;
        this.render();
      } else if (target.id === 'show-login') {
        e.preventDefault();
        this.state.showRegisterForm = false;
        this.state.showLoginForm = true;
        this.render();
      } else if (target.id === 'skip-login') {
        e.preventDefault();
        this.state.showLoginForm = false;
        this.render();
      } else if (target.id === 'add-entry-btn') {
        this.showAddForm();
      } else if (target.id === 'storage-settings-btn') {
        this.showStorageSettings();
      } else if (target.id === 'save-storage-settings-btn') {
        this.saveStorageSettings();
      } else if (target.id === 'cancel-storage-settings-btn') {
        this.hideStorageSettings();
      } else if (target.id === 'test-storage-btn') {
        this.testStorage();
      } else if (target.id === 'force-sync-btn') {
        this.forceSync();
      } else if (target.id === 'verify-cloud-btn') {
        this.verifyCloudStorage();
      } else if (target.id === 'reset-vault-btn') {
        this.handleResetVault();
      } else if (target.classList.contains('storage-option')) {
        this.selectStorageMode(target.dataset.mode);
      } else if (target.id === 'save-entry-btn') {
        this.saveEntry();
      } else if (target.id === 'cancel-add-btn') {
        this.hideAddForm();
      } else if (target.id === 'generate-password-btn') {
        this.generatePassword();
      } else if (action === 'copy' && entryId) {
        this.copyPassword(entryId);
      } else if (action === 'fill' && entryId) {
        this.fillPassword(entryId);
      } else if (action === 'delete' && entryId) {
        this.deleteEntry(entryId);
      }
    });

    app.addEventListener('keypress', (e) => {
      if (e.key === 'Enter') {
        if (e.target.id === 'master-password') {
          this.handleUnlock();
        } else if (e.target.id === 'login-password') {
          this.handleLogin();
        } else if (e.target.id === 'register-confirm') {
          this.handleRegister();
        } else if (e.target.id === 'password-input') {
          this.saveEntry();
        }
      }
    });

    app.addEventListener('input', (e) => {
      if (e.target.id === 'search-input') {
        this.state.searchTerm = e.target.value;
        this.render();
      }
    });
  }

  async handleUnlock() {
    const passwordInput = document.getElementById('master-password');
    const password = passwordInput.value;
    
    if (!password) {
      this.state.error = 'Please enter a password';
      this.render();
      return;
    }

    if (password.length < 8) {
      this.state.error = 'Master password must be at least 8 characters';
      this.render();
      return;
    }
    
    if (!this.wasm) {
      this.state.error = 'Encryption module not loaded';
      this.render();
      return;
    }
    
    try {
      this.state.error = null;
      
      // Generate or retrieve salt for this user
      let salt = await this.getSalt();
      console.log('Retrieved salt:', salt ? 'exists' : 'null', salt ? salt.length : 0);
      
      if (!salt) {
        // Generate salt based on implementation type
        if (this.wasmType === 'rust' && this.wasm.generate_salt) {
          salt = this.wasm.generate_salt();
        } else if (this.wasmType === 'js' && this.wasm.generateSalt) {
          salt = this.wasm.generateSalt();
        } else {
          // Fallback salt generation
          salt = Array.from(crypto.getRandomValues(new Uint8Array(32)));
        }
        await this.saveSalt(salt);
        console.log('Generated new salt for first-time user, length:', salt.length);
      }
      
      // Convert salt to appropriate format
      const saltArray = new Uint8Array(salt);
      
      // Check if this is first-time setup or existing user
      const userData = await chrome.storage.local.get(['passmann_master_hash', 'passmann_initialized']);
      
      if (!userData.passmann_initialized || !userData.passmann_master_hash) {
        // First-time setup - create master password hash
        const masterHash = await this.createMasterPasswordHash(password, saltArray);
        
        // Store the hash securely
        await chrome.storage.local.set({ 
          passmann_master_hash: Array.from(masterHash),
          passmann_initialized: true,
          passmann_session_start: Date.now()
        });
        
        this.state.success = 'Master password created successfully!';
        this.state.masterPassword = password;
        this.state.isLocked = false;
        
        // Start session monitoring
        this.startSessionMonitoring();
      } else {
        // Existing user - verify master password
        const storedHash = new Uint8Array(userData.passmann_master_hash);
        const inputHash = await this.createMasterPasswordHash(password, saltArray);
        
        console.log('Password verification:', {
          storedHashLength: storedHash.length,
          inputHashLength: inputHash.length,
          saltLength: saltArray.length
        });
        
        // Compare hashes securely
        if (this.secureCompare(storedHash, inputHash)) {
          // Password correct - unlock vault
          console.log('Master password verified successfully');
          
          // Reset failed attempts counter on successful login
          await chrome.storage.local.remove(['passmann_failed_attempts']);
          
          await chrome.storage.local.set({ 
            passmann_session_start: Date.now()
          });
          
          this.state.masterPassword = password;
          this.state.isLocked = false;
          this.state.success = 'Vault unlocked successfully!';
          
          // Start session monitoring
          this.startSessionMonitoring();
        } else {
          // Password incorrect
          console.error('Master password verification failed');
          console.log('Hash comparison failed - password is incorrect');
          
          // Check if user has multiple failed attempts and offer reset option
          const failedAttempts = await chrome.storage.local.get(['passmann_failed_attempts']);
          let attempts = (failedAttempts.passmann_failed_attempts || 0) + 1;
          await chrome.storage.local.set({ passmann_failed_attempts: attempts });
          this.state.failedAttempts = attempts; // Update state for UI
          
          if (attempts >= 3) {
            this.state.error = `Invalid master password (${attempts} failed attempts). If you've forgotten your password, you may need to reset your vault (this will delete all stored data).`;
            console.warn(`User has ${attempts} failed login attempts`);
          } else {
            this.state.error = `Invalid master password (attempt ${attempts}/3)`;
          }
          
          this.render();
          return;
        }
      }
      
      // Initialize WASM with the correct password and load existing vault
      let unlockResult = false;
      let vaultData = null; // Declare vaultData in outer scope
      try {
        // Use unlock_vault method for proper vault initialization
        if (this.wasmType === 'rust') {
          // Load existing vault data if available
          vaultData = await this.storageManager.loadVault();
          let encryptedVaultData = null;
          
          if (vaultData && vaultData.encryptedData) {
            encryptedVaultData = Array.from(vaultData.encryptedData);
            console.log('Found existing vault data to decrypt, size:', encryptedVaultData.length);
          } else {
            console.log('No existing vault data found, creating new vault');
          }
          
          // Debug WASM methods available
          console.log('Available WASM methods:', Object.getOwnPropertyNames(this.wasm));
          
          // Use unlock_vault method which properly initializes the vault
          if (this.wasm.unlock_vault) {
            console.log('Attempting unlock_vault with:', {
              passwordLength: password.length,
              saltLength: saltArray.length,
              hasEncryptedData: !!encryptedVaultData,
              encryptedDataLength: encryptedVaultData ? encryptedVaultData.length : 0
            });
            
            // Try unlock_vault method
            unlockResult = this.wasm.unlock_vault(password, saltArray, encryptedVaultData);
            console.log('WASM unlock_vault result:', unlockResult, typeof unlockResult);
            
            // If unlock_vault fails and we have an existing user, try simple unlock method
            if (!unlockResult && userData.passmann_initialized && this.wasm.unlock) {
              console.log('unlock_vault failed, trying simple unlock method as fallback');
              try {
                unlockResult = this.wasm.unlock(password, saltArray);
                console.log('Fallback unlock result:', unlockResult, typeof unlockResult);
                
                // If simple unlock worked, we can load the encrypted data separately
                if (unlockResult && encryptedVaultData) {
                  console.log('Simple unlock worked, attempting to load vault data');
                  if (this.wasm.load_vault_data) {
                    const loadResult = this.wasm.load_vault_data(encryptedVaultData);
                    console.log('Load vault data result:', loadResult);
                  }
                }
              } catch (fallbackError) {
                console.error('Fallback unlock also failed:', fallbackError);
              }
            }
            
          } else if (this.wasm.unlock) {
            // Fallback to unlock method if unlock_vault doesn't exist
            console.log('Attempting fallback unlock method');
            unlockResult = this.wasm.unlock(password, saltArray);
            console.log('WASM unlock result (fallback):', unlockResult, typeof unlockResult);
          } else {
            throw new Error('No unlock method available in WASM module');
          }
          
          // Verify WASM state after unlock
          if (this.wasm.is_unlocked) {
            const isUnlocked = this.wasm.is_unlocked();
            console.log('WASM is_unlocked:', isUnlocked);
            if (!isUnlocked && userData.passmann_initialized) {
              throw new Error('WASM reports vault is still locked after unlock attempt');
            }
          }
          
          if (this.wasm.get_entries_count) {
            console.log('WASM entries count after unlock:', this.wasm.get_entries_count());
          }
          
          // Debug WASM state after unlock
          this.debugWasmState();
        } else {
          // JavaScript fallback
          unlockResult = await this.wasm.unlock(password, saltArray);
        }
        
      } catch (unlockError) {
        console.error('WASM unlock failed:', unlockError);
        console.error('WASM unlock error details:', {
          wasmType: this.wasmType,
          wasmMethods: this.wasm ? Object.getOwnPropertyNames(this.wasm) : 'no wasm',
          passwordLength: password.length,
          saltLength: saltArray.length,
          hasVaultData: !!(vaultData && vaultData.encryptedData)
        });
        this.state.error = 'Failed to initialize encryption module: ' + unlockError.message;
        this.render();
        return;
      }

      // Check unlock result - for new users, false is acceptable
      console.log('Checking unlock result:', {
        unlockResult: unlockResult,
        unlockResultType: typeof unlockResult,
        isInitialized: userData.passmann_initialized,
        hasMasterHash: !!userData.passmann_master_hash
      });
      
      if (!unlockResult && userData.passmann_initialized) {
        // For existing users, try to verify if this is a password issue or data corruption
        console.error('Unlock failed for existing user - analyzing failure...');
        
        // Check if vault data exists and is valid
        if (vaultData && vaultData.encryptedData && vaultData.encryptedData.length > 0) {
          console.log('Vault data exists, likely password mismatch');
          this.state.error = 'Invalid master password. Please check your password and try again.';
        } else {
          console.log('No vault data found, might be first time with cloud storage');
          // If no local vault data but user is initialized, might be using cloud storage
          // Let's allow the unlock to proceed and try to load from cloud
          console.log('Allowing unlock to proceed - might be cloud-only user');
          unlockResult = true;
        }
        
        if (!unlockResult) {
          this.render();
          return;
        }
      } else if (!unlockResult && !userData.passmann_initialized) {
        console.log('New user setup, unlock result false is expected');
        unlockResult = true; // Allow new user setup to proceed
      } else if (unlockResult) {
        console.log('Unlock successful!');
      }
      
      // Load existing vault entries after successful unlock
      await this.loadVaultEntries();
      
      // Save session data for 60-minute unlock period
      const sessionData = {
        passmann_session_start: Date.now(),
        passmann_session_key: crypto.randomUUID(),
        passmann_master_password_hash: await this.hashPassword(password),
        passmann_last_activity: Date.now()
      };
      
      await chrome.storage.session.set(sessionData);
      console.log('Session data saved for 60-minute unlock period');
      
      this.state.masterPassword = password;
      this.state.isLocked = false;
      this.state.error = null;
      this.state.success = 'Vault unlocked successfully!';
      
      await this.saveState();
      this.render();
      
      // Start session monitoring
      this.startSessionMonitoring();
      
    } catch (error) {
      console.error('Unlock error:', error);
      this.state.error = 'Failed to unlock vault: ' + error.message;
      this.render();
    }
  }

  async handleLock() {
    if (this.wasm) {
      try {
        if (this.wasmType === 'rust') {
          // WASM version
          if (this.wasm.is_unlocked && this.wasm.is_unlocked()) {
            this.wasm.lock();
          }
        } else if (this.wasmType === 'js') {
          // JavaScript version
          if (this.wasm.isUnlocked && this.wasm.isUnlocked()) {
            this.wasm.lock();
          }
        }
      } catch (error) {
        console.warn('Error during WASM lock:', error);
      }
    }
    
    // Use our secure locking system
    this.lockVault();
  }

  async handleLogin() {
    const email = document.getElementById('login-email').value.trim();
    const password = document.getElementById('login-password').value;
    
    if (!email || !password) {
      this.state.error = 'Please fill in all fields';
      this.render();
      return;
    }
    
    if (!email.includes('@')) {
      this.state.error = 'Please enter a valid email address';
      this.render();
      return;
    }
    
    await this.loginWithCloud(email, password);
  }

  async handleRegister() {
    const email = document.getElementById('register-email').value.trim();
    const password = document.getElementById('register-password').value;
    const confirmPassword = document.getElementById('register-confirm').value;
    
    if (!email || !password || !confirmPassword) {
      this.state.error = 'Please fill in all fields';
      this.render();
      return;
    }
    
    if (!email.includes('@')) {
      this.state.error = 'Please enter a valid email address';
      this.render();
      return;
    }
    
    await this.registerWithCloud(email, password, confirmPassword);
  }

  // Cloud Authentication Methods
  async loginWithCloud(email, password) {
    try {
      this.state.loading = true;
      this.render();

      // Authenticate with cloud service
      const authResponse = await this.cloudAuth(email, password);
      
      if (authResponse.success) {
        // Store user session
        await chrome.storage.local.set({
          passmann_user: {
            email: email,
            token: authResponse.token,
            userId: authResponse.userId,
            loginTime: Date.now()
          },
          passmann_cloud_enabled: true
        });

        this.state.user = {
          email: email,
          userId: authResponse.userId
        };
        this.state.cloudSyncEnabled = true;
        this.state.showLoginForm = false;
        this.state.success = 'Logged in successfully!';
        
        // Sync vault from cloud
        await this.syncFromCloud();
        
      } else {
        this.state.error = authResponse.error || 'Login failed';
      }
    } catch (error) {
      console.error('Login error:', error);
      this.state.error = 'Failed to login: ' + error.message;
    } finally {
      this.state.loading = false;
      this.render();
    }
  }

  async registerWithCloud(email, password, confirmPassword) {
    try {
      if (password !== confirmPassword) {
        this.state.error = 'Passwords do not match';
        this.render();
        return;
      }

      if (password.length < 12) {
        this.state.error = 'Password must be at least 12 characters';
        this.render();
        return;
      }

      this.state.loading = true;
      this.render();

      // Register with cloud service
      const registerResponse = await this.cloudRegister(email, password);
      
      if (registerResponse.success) {
        this.state.success = 'Account created successfully! Please login.';
        this.state.showRegisterForm = false;
        this.state.showLoginForm = true;
      } else {
        this.state.error = registerResponse.error || 'Registration failed';
      }
    } catch (error) {
      console.error('Registration error:', error);
      this.state.error = 'Failed to register: ' + error.message;
    } finally {
      this.state.loading = false;
      this.render();
    }
  }

  async cloudAuth(email, password) {
    // This would connect to your cloud authentication service
    // For now, simulate the authentication
    return new Promise((resolve) => {
      setTimeout(() => {
        // Simulate successful authentication
        resolve({
          success: true,
          token: 'mock_jwt_token_' + Date.now(),
          userId: 'user_' + email.replace('@', '_').replace('.', '_'),
          email: email
        });
      }, 1000);
    });
  }

  async cloudRegister(email, password) {
    // This would connect to your cloud registration service
    return new Promise((resolve) => {
      setTimeout(() => {
        // Simulate successful registration
        resolve({
          success: true,
          message: 'Account created successfully'
        });
      }, 1000);
    });
  }

  async syncFromCloud() {
    try {
      if (!this.state.cloudSyncEnabled) return;

      // Get stored user data
      const userData = await chrome.storage.local.get(['passmann_user']);
      if (!userData.passmann_user) return;

      // Simulate cloud sync
      console.log('Syncing vault from cloud for user:', userData.passmann_user.email);
      
      // Here you would fetch the encrypted vault from your cloud service
      // For now, just update the last sync time
      this.state.lastSync = new Date().toLocaleString();
      
      await chrome.storage.local.set({
        passmann_last_sync: Date.now()
      });

    } catch (error) {
      console.error('Cloud sync error:', error);
    }
  }

  async syncToCloud() {
    try {
      if (!this.state.cloudSyncEnabled) return;

      const userData = await chrome.storage.local.get(['passmann_user']);
      if (!userData.passmann_user) return;

      // Here you would upload the encrypted vault to your cloud service
      console.log('Syncing vault to cloud for user:', userData.passmann_user.email);
      
      this.state.lastSync = new Date().toLocaleString();
      await chrome.storage.local.set({
        passmann_last_sync: Date.now()
      });

    } catch (error) {
      console.error('Cloud sync error:', error);
    }
  }

  async logoutFromCloud() {
    try {
      // Clear user session
      await chrome.storage.local.remove([
        'passmann_user',
        'passmann_cloud_enabled',
        'passmann_last_sync'
      ]);

      this.state.user = null;
      this.state.cloudSyncEnabled = false;
      this.state.showLoginForm = true;
      this.state.success = 'Logged out successfully';
      this.render();

    } catch (error) {
      console.error('Logout error:', error);
      this.state.error = 'Failed to logout: ' + error.message;
      this.render();
    }
  }

  async checkCloudSession() {
    try {
      const userData = await chrome.storage.local.get(['passmann_user']);
      
      if (userData.passmann_user) {
        // Check if session is still valid (24 hours)
        const sessionAge = Date.now() - userData.passmann_user.loginTime;
        const maxSessionAge = 24 * 60 * 60 * 1000; // 24 hours

        if (sessionAge < maxSessionAge) {
          this.state.user = {
            email: userData.passmann_user.email,
            userId: userData.passmann_user.userId
          };
          this.state.cloudSyncEnabled = true;
          this.state.showLoginForm = false;
          
          // Get last sync time
          const syncData = await chrome.storage.local.get(['passmann_last_sync']);
          if (syncData.passmann_last_sync) {
            this.state.lastSync = new Date(syncData.passmann_last_sync).toLocaleString();
          }
          
          return true;
        } else {
          // Session expired, logout
          await this.logoutFromCloud();
        }
      }
      
      return false;
    } catch (error) {
      console.error('Session check error:', error);
      return false;
    }
  }

  showAddForm() {
    this.state.showAddForm = true;
    this.render();
  }

  hideAddForm() {
    this.state.showAddForm = false;
    this.render();
  }

  // Storage settings methods
  showStorageSettings() {
    this.state.showStorageSettings = true;
    this.render();
  }

  hideStorageSettings() {
    this.state.showStorageSettings = false;
    this.render();
  }

  selectStorageMode(mode) {
    console.log('Selecting storage mode:', mode);
    this.state.storageMode = mode;
    if (this.wasm && this.wasmType === 'rust') {
      this.wasm.set_storage_mode(mode);
    }
    console.log('Storage mode set to:', this.state.storageMode);
    this.render();
  }

  async saveStorageSettings() {
    try {
      // Save storage mode preference
      await chrome.storage.local.set({
        passmann_storage_mode: this.state.storageMode
      });

      // If cloud mode, save cloud configuration
      if (this.state.storageMode === 'cloud') {
        const email = document.getElementById('cloud-email')?.value;
        const serverUrl = document.getElementById('cloud-server')?.value;
        
        if (email && serverUrl) {
          const cloudConfig = {
            email,
            serverUrl,
            connected: false,
            timestamp: Date.now()
          };
          
          await this.storageManager.saveCloudConfig(cloudConfig);
          console.log('Cloud configuration saved');
        }
      }

      // Force save current vault data with new storage mode
      await this.saveState();
      
      this.state.success = `Storage settings saved! Mode: ${this.state.storageMode.toUpperCase()}`;
      this.hideStorageSettings();
    } catch (error) {
      console.error('Failed to save storage settings:', error);
      this.state.error = 'Failed to save storage settings: ' + error.message;
      this.render();
    }
  }

  async testStorage() {
    try {
      this.state.success = 'Testing storage...';
      this.render();
      
      const healthCheck = await this.storageManager.performStorageHealthCheck();
      const availableStorage = Object.entries(healthCheck)
        .filter(([key, value]) => value)
        .map(([key]) => key)
        .join(', ');
        
      this.state.success = `Storage test completed! Available: ${availableStorage}`;
      this.render();
    } catch (error) {
      console.error('Storage test failed:', error);
      this.state.error = 'Storage test failed: ' + error.message;
      this.render();
    }
  }

  async forceSync() {
    try {
      this.state.success = 'Syncing vault data...';
      this.render();
      
      // Save current vault to all available storage locations
      if (!this.state.isLocked && this.wasm && this.wasmType === 'rust') {
        const encryptedVault = this.wasm.encrypt_vault();
        if (encryptedVault) {
          const salt = await this.getSalt();
          if (salt) {
            const success = await this.storageManager.saveVault(new Uint8Array(encryptedVault), salt);
            if (success) {
              this.state.lastSync = Date.now();
              this.state.success = 'Vault synchronized successfully!';
            } else {
              this.state.error = 'Sync failed - could not save to any storage location';
            }
          }
        }
      } else {
        this.state.error = 'Cannot sync - vault is locked or not initialized';
      }
      
      this.render();
    } catch (error) {
      console.error('Force sync failed:', error);
      this.state.error = 'Sync failed: ' + error.message;
      this.render();
    }
  }

  async verifyCloudStorage() {
    try {
      // Update status to show we're checking
      const cloudStatusElement = document.getElementById('cloud-status-text');
      if (cloudStatusElement) {
        cloudStatusElement.textContent = '🔄 Checking cloud connection...';
        cloudStatusElement.style.color = '#3498db';
      }

      // Check if cloud storage is configured
      const cloudConfig = await this.storageManager.getCloudConfig();
      if (!cloudConfig || !cloudConfig.serverUrl || !cloudConfig.authToken) {
        if (cloudStatusElement) {
          cloudStatusElement.textContent = '❌ Cloud not configured - please login first';
          cloudStatusElement.style.color = '#e74c3c';
        }
        return;
      }

      // Try to load data from cloud to verify connection
      const cloudData = await this.storageManager.loadFromCloud();
      
      if (cloudData) {
        // Cloud storage is working and has data
        if (cloudStatusElement) {
          cloudStatusElement.textContent = '✅ Cloud storage verified - data found!';
          cloudStatusElement.style.color = '#27ae60';
        }
        this.state.success = 'Cloud storage verification successful!';
      } else {
        // Try to save a test to see if cloud is reachable
        try {
          // Save current vault if available
          if (!this.state.isLocked && this.wasm && this.wasmType === 'rust') {
            const encryptedVault = this.wasm.encrypt_vault();
            if (encryptedVault) {
              const salt = await this.getSalt();
              if (salt) {
                const saveSuccess = await this.storageManager.saveToCloud(new Uint8Array(encryptedVault), salt);
                if (saveSuccess) {
                  if (cloudStatusElement) {
                    cloudStatusElement.textContent = '✅ Cloud storage verified - connection working!';
                    cloudStatusElement.style.color = '#27ae60';
                  }
                  this.state.success = 'Cloud storage connection verified!';
                } else {
                  if (cloudStatusElement) {
                    cloudStatusElement.textContent = '❌ Cloud storage not responding';
                    cloudStatusElement.style.color = '#e74c3c';
                  }
                  this.state.error = 'Cloud storage not responding';
                }
              }
            }
          } else {
            if (cloudStatusElement) {
              cloudStatusElement.textContent = '⚠️ Cloud configured but no data to verify with';
              cloudStatusElement.style.color = '#f39c12';
            }
            this.state.success = 'Cloud configuration appears valid (no data to test with)';
          }
        } catch (testError) {
          if (cloudStatusElement) {
            cloudStatusElement.textContent = '❌ Cloud storage connection failed';
            cloudStatusElement.style.color = '#e74c3c';
          }
          this.state.error = 'Cloud storage connection failed: ' + testError.message;
        }
      }
      
      this.render();
    } catch (error) {
      console.error('Cloud verification failed:', error);
      const cloudStatusElement = document.getElementById('cloud-status-text');
      if (cloudStatusElement) {
        cloudStatusElement.textContent = '❌ Verification failed: ' + error.message;
        cloudStatusElement.style.color = '#e74c3c';
      }
      this.state.error = 'Cloud verification failed: ' + error.message;
      this.render();
    }
  }

  async handleResetVault() {
    // Show confirmation dialog
    const confirmReset = confirm(
      "⚠️ WARNING: This will permanently delete ALL stored passwords and reset your vault.\n\n" +
      "This action cannot be undone!\n\n" +
      "Are you absolutely sure you want to proceed?"
    );
    
    if (!confirmReset) {
      return;
    }
    
    // Double confirmation for safety
    const doubleConfirm = confirm(
      "🔥 FINAL WARNING: You are about to delete ALL your passwords!\n\n" +
      "This will:\n" +
      "• Delete all stored passwords\n" +
      "• Clear your master password\n" +
      "• Reset all settings\n" +
      "• Clear cloud sync data\n\n" +
      "Click OK to proceed with complete vault reset."
    );
    
    if (!doubleConfirm) {
      return;
    }
    
    try {
      console.log('User requested vault reset - clearing all data');
      
      // Clear all local storage data
      await chrome.storage.local.clear();
      await chrome.storage.session.clear();
      
      // Reset WASM state if available
      if (this.wasm && this.wasmType === 'rust') {
        try {
          if (this.wasm.reset_vault) {
            this.wasm.reset_vault();
          }
        } catch (wasmError) {
          console.warn('WASM reset failed:', wasmError);
        }
      }
      
      // Reset application state
      this.state = {
        isLocked: true,
        entries: [],
        searchTerm: '',
        showAddForm: false,
        showLoginForm: true,
        showRegisterForm: false,
        showSettings: false,
        showStorageSettings: false,
        masterPassword: '',
        error: null,
        success: 'Vault has been completely reset. You can now set up a new master password.',
        loading: false,
        user: null,
        cloudSyncEnabled: false,
        lastSync: null,
        storageMode: 'local',
        failedAttempts: 0
      };
      
      // Stop session monitoring
      this.stopSessionMonitoring();
      
      console.log('Vault reset completed successfully');
      this.render();
      
    } catch (error) {
      console.error('Vault reset failed:', error);
      this.state.error = 'Failed to reset vault: ' + error.message;
      this.render();
    }
  }

  async saveEntry() {
    const service = document.getElementById('service-input').value.trim();
    const username = document.getElementById('username-input').value.trim();
    const password = document.getElementById('password-input').value;
    
    if (!service || !username || !password) {
      this.state.error = 'Please fill in all fields';
      this.render();
      return;
    }
    
    console.log('Saving entry:', { service, username, passwordLength: password.length });
    
    const entry = {
      id: Date.now().toString(),
      service,
      username,
      password,
      created: new Date().toISOString()
    };
    
    // Add entry to state
    this.state.entries.push(entry);
    console.log('Entry added to state. Total entries:', this.state.entries.length);
    
    // Save entry to WASM vault if available
    if (this.wasm && this.wasmType === 'rust' && !this.state.isLocked) {
      try {
        console.log('Attempting to add entry to WASM vault...');
        // Use the correct WASM method signature: add_entry(service, username, password, url, notes)
        const success = this.wasm.add_entry(entry.service, entry.username, entry.password, null, null);
        console.log('WASM add_entry result:', success);
        if (success) {
          console.log('Entry added to WASM vault successfully');
          console.log('WASM entries count after add:', this.wasm.get_entries_count ? this.wasm.get_entries_count() : 'Unknown');
        } else {
          console.warn('Failed to add entry to WASM vault');
        }
      } catch (wasmError) {
        console.error('WASM add entry error:', wasmError);
      }
    } else {
      console.log('WASM not available or vault locked:', {
        hasWasm: !!this.wasm,
        wasmType: this.wasmType,
        isLocked: this.state.isLocked
      });
    }
    
    // Save the updated state
    console.log('Saving state...');
    await this.saveState();
    
    this.state.success = 'Entry saved successfully!';
    this.hideAddForm();
    this.render();
  }

  generatePassword() {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*';
    let password = '';
    for (let i = 0; i < 16; i++) {
      password += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    const passwordInput = document.getElementById('password-input');
    if (passwordInput) {
      passwordInput.value = password;
    }
  }

  // Check for pending password saves from content script
  async checkPendingSaves() {
    try {
      const result = await chrome.storage.session.get(['pendingSave']);
      if (result.pendingSave) {
        const pendingData = result.pendingSave;
        
        // Check if the save request is recent (within 5 minutes)
        const isRecent = (Date.now() - pendingData.timestamp) < 300000; // 5 minutes
        
        if (isRecent && !this.state.isLocked) {
          // Auto-add the password if the vault is unlocked
          const newEntry = {
            id: Date.now().toString(),
            service: pendingData.site, // Use 'service' field for consistency
            username: pendingData.username,
            password: pendingData.password,
            url: pendingData.url,
            created: new Date().toISOString()
          };
          
          // Add entry to state
          this.state.entries.push(newEntry);
          
          // Save entry to WASM vault if available
          if (this.wasm && this.wasmType === 'rust') {
            try {
              const success = this.wasm.add_entry(newEntry.service, newEntry.username, newEntry.password, newEntry.url || null, null);
              if (success) {
                console.log('Pending entry added to WASM vault successfully');
              }
            } catch (wasmError) {
              console.error('WASM add entry error for pending save:', wasmError);
            }
          }
          
          await this.saveState();
          
          this.state.success = `Password for ${pendingData.site} saved automatically!`;
          this.render();
          
          // Clear the pending save
          await chrome.storage.session.remove(['pendingSave']);
        }
      }
    } catch (error) {
      console.error('Error checking pending saves:', error);
    }
  }

  async copyPassword(entryId) {
    const entry = this.state.entries.find(e => e.id === entryId);
    if (entry) {
      try {
        await navigator.clipboard.writeText(entry.password);
        this.state.success = 'Password copied to clipboard!';
        this.render();
      } catch (error) {
        console.error('Failed to copy password:', error);
        this.state.error = 'Failed to copy password';
        this.render();
      }
    }
  }

  async fillPassword(entryId) {
    const entry = this.state.entries.find(e => e.id === entryId);
    if (entry) {
      try {
        // Get current active tab
        const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
        if (tab) {
          // Send fill credentials message to content script
          chrome.tabs.sendMessage(tab.id, {
            action: 'fillCredentials',
            data: {
              username: entry.username,
              password: entry.password
            }
          }, (response) => {
            if (chrome.runtime.lastError) {
              console.error('Failed to fill credentials:', chrome.runtime.lastError);
              this.state.error = 'Failed to auto-fill on this page';
              this.render();
            } else if (response && response.success) {
              this.state.success = 'Credentials filled successfully!';
              this.render();
              // Close popup after successful fill
              setTimeout(() => {
                window.close();
              }, 1000);
            } else {
              this.state.error = 'Could not find form fields to fill';
              this.render();
            }
          });
        }
      } catch (error) {
        console.error('Failed to fill password:', error);
        this.state.error = 'Failed to auto-fill credentials';
        this.render();
      }
    }
  }

  deleteEntry(entryId) {
    if (confirm('Are you sure you want to delete this entry?')) {
      this.state.entries = this.state.entries.filter(e => e.id !== entryId);
      this.saveState();
      this.state.success = 'Entry deleted successfully!';
      this.render();
    }
  }

  escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }
}

// Enhanced Animation Manager for MetaMask-style effects
class AnimationManager {
  static fadeInUp(element, delay = 0) {
    element.style.opacity = '0';
    element.style.transform = 'translateY(20px)';
    element.style.transition = 'all 0.6s cubic-bezier(0.4, 0, 0.2, 1)';
    
    setTimeout(() => {
      element.style.opacity = '1';
      element.style.transform = 'translateY(0)';
    }, delay);
  }

  static slideInFromRight(element, delay = 0) {
    element.style.opacity = '0';
    element.style.transform = 'translateX(100%)';
    element.style.transition = 'all 0.5s cubic-bezier(0.4, 0, 0.2, 1)';
    
    setTimeout(() => {
      element.style.opacity = '1';
      element.style.transform = 'translateX(0)';
    }, delay);
  }

  static pulseSuccess(element) {
    element.classList.add('pulse');
    setTimeout(() => {
      element.classList.remove('pulse');
    }, 2000);
  }

  static bounceNotification(element) {
    element.classList.add('bounce');
    setTimeout(() => {
      element.classList.remove('bounce');
    }, 600);
  }

  static scaleClick(element) {
    element.style.transform = 'scale(0.95)';
    element.style.transition = 'transform 0.1s ease';
    
    setTimeout(() => {
      element.style.transform = 'scale(1)';
    }, 100);
  }
}

// Initialize app when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
  window.passmannApp = new PassMannApp();
  
  // Add entrance animations
  setTimeout(() => {
    const container = document.querySelector('.container');
    if (container) {
      AnimationManager.fadeInUp(container, 200);
    }
  }, 100);
  
  // Add click animations to all buttons
  document.addEventListener('click', (e) => {
    if (e.target.matches('.btn, .btn-small')) {
      AnimationManager.scaleClick(e.target);
    }
  });
});

// 3D Fox Mascot Mouse Tracking function for PassMannApp
PassMannApp.prototype.initializeFoxMascot = function() {
  const fox = document.querySelector('.fox-mascot');
  const leftEye = document.querySelector('.fox-eye.left');
  const rightEye = document.querySelector('.fox-eye.right');
  
  if (!fox || !leftEye || !rightEye) return;
  
  document.addEventListener('mousemove', (event) => {
    const container = document.querySelector('.container');
    if (!container) return;
    
    const containerRect = container.getBoundingClientRect();
    const foxRect = fox.getBoundingClientRect();
    
    // Calculate mouse position relative to container
    const mouseX = event.clientX - containerRect.left;
    const mouseY = event.clientY - containerRect.top;
    
    // Calculate fox center position relative to container
    const foxCenterX = (foxRect.left + foxRect.width / 2) - containerRect.left;
    const foxCenterY = (foxRect.top + foxRect.height / 2) - containerRect.top;
    
    // Calculate angle and distance to mouse
    const deltaX = mouseX - foxCenterX;
    const deltaY = mouseY - foxCenterY;
    const angle = Math.atan2(deltaY, deltaX);
    const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
    
    // Limit eye movement range
    const maxEyeMovement = 3;
    const eyeMovementX = Math.cos(angle) * Math.min(distance / 20, maxEyeMovement);
    const eyeMovementY = Math.sin(angle) * Math.min(distance / 20, maxEyeMovement);
    
    // Apply eye movement
    leftEye.style.transform = `translate(${eyeMovementX}px, ${eyeMovementY}px)`;
    rightEye.style.transform = `translate(${eyeMovementX}px, ${eyeMovementY}px)`;
    
    // Subtle head rotation based on mouse position
    const headRotation = Math.max(-15, Math.min(15, deltaX / 10));
    fox.style.transform = `rotateY(${headRotation}deg)`;
  });
  
  // Reset fox position when mouse leaves container
  document.addEventListener('mouseleave', () => {
    leftEye.style.transform = 'translate(0, 0)';
    rightEye.style.transform = 'translate(0, 0)';
    fox.style.transform = 'rotateY(0deg)';
  });
};
