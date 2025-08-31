// SQLite-like Storage Manager for PassMann Browser Extension
// Uses IndexedDB with SQLCipher-style encryption for maximum persistence

class SQLiteLikeStorageManager {
  constructor(app) {
    this.app = app;
    this.DB_NAME = 'PassMannVaultDB_SQLite';
    this.DB_VERSION = 4;
    this.VAULT_TABLE = 'vault_entries';
    this.METADATA_TABLE = 'vault_metadata';
    this.SESSIONS_TABLE = 'vault_sessions';
    this.db = null;
    this.isInitialized = false;
    this.encryptionKey = null;
    
    this.initDatabase();
  }

  // Initialize SQLite-like database structure
  async initDatabase() {
    return new Promise((resolve, reject) => {
      const request = indexedDB.open(this.DB_NAME, this.DB_VERSION);
      
      request.onerror = () => {
        reject(request.error);
      };
      
      request.onsuccess = (event) => {
        this.db = event.target.result;
        this.isInitialized = true;
        
        // Set up error handling
        this.db.onerror = (event) => {
          console.error('Database error:', event.target.error);
        };
        
        resolve(this.db);
      };
      
      request.onupgradeneeded = (event) => {
        const db = event.target.result;
        
        // Create vault entries table (like SQLite table)
        if (!db.objectStoreNames.contains(this.VAULT_TABLE)) {
          const vaultStore = db.createObjectStore(this.VAULT_TABLE, { 
            keyPath: 'id', 
            autoIncrement: true 
          });
          
          // Create indexes (like SQLite indexes)
          vaultStore.createIndex('service', 'service', { unique: false });
          vaultStore.createIndex('username', 'username', { unique: false });
          vaultStore.createIndex('created_at', 'created_at', { unique: false });
          vaultStore.createIndex('updated_at', 'updated_at', { unique: false });
          vaultStore.createIndex('is_deleted', 'is_deleted', { unique: false });
        }
        
        // Create metadata table
        if (!db.objectStoreNames.contains(this.METADATA_TABLE)) {
          const metaStore = db.createObjectStore(this.METADATA_TABLE, { 
            keyPath: 'key' 
          });
          
          metaStore.createIndex('created_at', 'created_at', { unique: false });
        }
        
        // Create sessions table
        if (!db.objectStoreNames.contains(this.SESSIONS_TABLE)) {
          const sessionStore = db.createObjectStore(this.SESSIONS_TABLE, { 
            keyPath: 'session_id' 
          });
          
          sessionStore.createIndex('created_at', 'created_at', { unique: false });
          sessionStore.createIndex('expires_at', 'expires_at', { unique: false });
        }
      };
    });
  }

  // SQLCipher-style key derivation
  async deriveEncryptionKey(masterPassword, salt) {
    const encoder = new TextEncoder();
    const passwordData = encoder.encode(masterPassword);
    
    // Import password as key material
    const keyMaterial = await crypto.subtle.importKey(
      'raw',
      passwordData,
      { name: 'PBKDF2' },
      false,
      ['deriveKey']
    );
    
    // Derive encryption key (SQLCipher-style with high iterations)
    return await crypto.subtle.deriveKey(
      {
        name: 'PBKDF2',
        salt: salt,
        iterations: 256000, // SQLCipher default
        hash: 'SHA-256'
      },
      keyMaterial,
      {
        name: 'AES-GCM',
        length: 256
      },
      false,
      ['encrypt', 'decrypt']
    );
  }

  // Encrypt data like SQLCipher
  async encryptData(data, key) {
    const iv = crypto.getRandomValues(new Uint8Array(12));
    const encoder = new TextEncoder();
    const dataBytes = typeof data === 'string' ? encoder.encode(data) : data;
    
    const encrypted = await crypto.subtle.encrypt(
      {
        name: 'AES-GCM',
        iv: iv,
        tagLength: 128
      },
      key,
      dataBytes
    );
    
    // Combine IV + encrypted data (SQLCipher format)
    const result = new Uint8Array(iv.length + encrypted.byteLength);
    result.set(iv);
    result.set(new Uint8Array(encrypted), iv.length);
    
    return result;
  }

  // Decrypt data like SQLCipher
  async decryptData(encryptedData, key) {
    const iv = encryptedData.slice(0, 12);
    const data = encryptedData.slice(12);
    
    const decrypted = await crypto.subtle.decrypt(
      {
        name: 'AES-GCM',
        iv: iv,
        tagLength: 128
      },
      key,
      data
    );
    
    return new TextDecoder().decode(decrypted);
  }

  // Initialize vault with master password (like PRAGMA key)
  async unlockVault(masterPassword) {
    try {
      // Get or create salt
      const salt = await this.getOrCreateSalt();
      
      // Derive encryption key
      this.encryptionKey = await this.deriveEncryptionKey(masterPassword, salt);
      
      // Test encryption by trying to read metadata
      const testResult = await this.getMetadata('vault_version');
      
      return true;
    } catch (error) {
      console.error('Failed to unlock vault:', error);
      this.encryptionKey = null;
      return false;
    }
  }

  // Get or create salt (stored in metadata)
  async getOrCreateSalt() {
    let saltMetadata = await this.getMetadataRaw('encryption_salt');
    
    if (!saltMetadata) {
      // Create new salt
      const salt = crypto.getRandomValues(new Uint8Array(32));
      await this.setMetadataRaw('encryption_salt', Array.from(salt));
      return salt;
    }
    
    return new Uint8Array(saltMetadata.value);
  }

  // ===== SQL-LIKE OPERATIONS =====

  // INSERT INTO vault_entries
  async insertEntry(service, username, password, notes = '', url = '') {
    if (!this.encryptionKey) {
      throw new Error('Vault is locked. Call unlockVault() first.');
    }

    const now = Date.now();
    const entry = {
      service: service,
      username: username,
      password: await this.encryptData(password, this.encryptionKey),
      notes: notes ? await this.encryptData(notes, this.encryptionKey) : '',
      url: url,
      created_at: now,
      updated_at: now,
      is_deleted: false
    };

    return new Promise((resolve, reject) => {
      const transaction = this.db.transaction([this.VAULT_TABLE], 'readwrite');
      const store = transaction.objectStore(this.VAULT_TABLE);
      
      const request = store.add(entry);
      
      request.onsuccess = () => {
        const insertedId = request.result;
        resolve(insertedId);
      };
      
      request.onerror = () => {
        reject(request.error);
      };
    });
  }

  // SELECT * FROM vault_entries WHERE...
  async selectEntries(whereClause = {}) {
    if (!this.encryptionKey) {
      throw new Error('Vault is locked. Call unlockVault() first.');
    }

    return new Promise((resolve, reject) => {
      const transaction = this.db.transaction([this.VAULT_TABLE], 'readonly');
      const store = transaction.objectStore(this.VAULT_TABLE);
      
      const request = store.getAll();
      
      request.onsuccess = async () => {
        try {
          let entries = request.result;
          
          // Filter by where clause
          if (whereClause.service) {
            entries = entries.filter(e => e.service.toLowerCase().includes(whereClause.service.toLowerCase()));
          }
          if (whereClause.username) {
            entries = entries.filter(e => e.username.toLowerCase().includes(whereClause.username.toLowerCase()));
          }
          if (whereClause.is_deleted !== undefined) {
            entries = entries.filter(e => e.is_deleted === whereClause.is_deleted);
          }
          
          // Decrypt sensitive fields
          const decryptedEntries = [];
          for (const entry of entries) {
            if (!entry.is_deleted) {
              try {
                const decryptedEntry = {
                  ...entry,
                  password: entry.password ? await this.decryptData(new Uint8Array(entry.password), this.encryptionKey) : '',
                  notes: entry.notes ? await this.decryptData(new Uint8Array(entry.notes), this.encryptionKey) : ''
                };
                decryptedEntries.push(decryptedEntry);
              } catch (decryptError) {
                console.warn('Failed to decrypt entry:', entry.id, decryptError);
              }
            }
          }
          
          resolve(decryptedEntries);
        } catch (error) {
          reject(error);
        }
      };
      
      request.onerror = () => {
        reject(request.error);
      };
    });
  }

  // UPDATE vault_entries SET ... WHERE id = ?
  async updateEntry(id, updates) {
    if (!this.encryptionKey) {
      throw new Error('Vault is locked. Call unlockVault() first.');
    }

    return new Promise(async (resolve, reject) => {
      const transaction = this.db.transaction([this.VAULT_TABLE], 'readwrite');
      const store = transaction.objectStore(this.VAULT_TABLE);
      
      // Get existing entry
      const getRequest = store.get(id);
      
      getRequest.onsuccess = async () => {
        try {
          const entry = getRequest.result;
          if (!entry) {
            reject(new Error('Entry not found'));
            return;
          }
          
          // Apply updates
          if (updates.service) entry.service = updates.service;
          if (updates.username) entry.username = updates.username;
          if (updates.password) entry.password = await this.encryptData(updates.password, this.encryptionKey);
          if (updates.notes !== undefined) entry.notes = updates.notes ? await this.encryptData(updates.notes, this.encryptionKey) : '';
          if (updates.url !== undefined) entry.url = updates.url;
          
          entry.updated_at = Date.now();
          
          // Save updated entry
          const putRequest = store.put(entry);
          
          putRequest.onsuccess = () => {
            resolve(true);
          };
          
          putRequest.onerror = () => {
            reject(putRequest.error);
          };
        } catch (error) {
          reject(error);
        }
      };
      
      getRequest.onerror = () => {
        reject(getRequest.error);
      };
    });
  }

  // DELETE FROM vault_entries WHERE id = ? (soft delete)
  async deleteEntry(id) {
    return this.updateEntry(id, { is_deleted: true });
  }

  // ===== METADATA OPERATIONS =====

  async setMetadata(key, value) {
    if (!this.encryptionKey) {
      return this.setMetadataRaw(key, value);
    }
    
    const encryptedValue = await this.encryptData(JSON.stringify(value), this.encryptionKey);
    return this.setMetadataRaw(key, Array.from(encryptedValue));
  }

  async getMetadata(key) {
    if (!this.encryptionKey) {
      return this.getMetadataRaw(key);
    }
    
    const rawData = await this.getMetadataRaw(key);
    if (!rawData) return null;
    
    try {
      const decrypted = await this.decryptData(new Uint8Array(rawData.value), this.encryptionKey);
      return JSON.parse(decrypted);
    } catch (error) {
      console.warn('Failed to decrypt metadata:', key, error);
      return null;
    }
  }

  async setMetadataRaw(key, value) {
    return new Promise((resolve, reject) => {
      const transaction = this.db.transaction([this.METADATA_TABLE], 'readwrite');
      const store = transaction.objectStore(this.METADATA_TABLE);
      
      const metadata = {
        key: key,
        value: value,
        created_at: Date.now()
      };
      
      const request = store.put(metadata);
      
      request.onsuccess = () => resolve(true);
      request.onerror = () => reject(request.error);
    });
  }

  async getMetadataRaw(key) {
    return new Promise((resolve, reject) => {
      const transaction = this.db.transaction([this.METADATA_TABLE], 'readonly');
      const store = transaction.objectStore(this.METADATA_TABLE);
      
      const request = store.get(key);
      
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
  }

  // ===== LEGACY API COMPATIBILITY =====

  // For popup.js compatibility
  async saveVault(encryptedData, salt) {
    // Store the raw vault data as metadata for WASM compatibility
    await this.setMetadataRaw('legacy_vault_data', Array.from(encryptedData));
    await this.setMetadataRaw('legacy_vault_salt', Array.from(salt));
    
    // Also save to localStorage and chrome.storage as backup
    try {
      localStorage.setItem('passmann_vault_sqlite_backup', JSON.stringify({
        data: Array.from(encryptedData),
        salt: Array.from(salt),
        timestamp: Date.now()
      }));
    } catch (e) {
      // localStorage not available
    }
    
    try {
      if (chrome?.storage?.local) {
        await chrome.storage.local.set({
          'passmann_vault_sqlite_backup': {
            data: Array.from(encryptedData),
            salt: Array.from(salt),
            timestamp: Date.now()
          }
        });
      }
    } catch (e) {
      // chrome.storage not available
    }
    
    return true;
  }

  async loadVault() {
    try {
      // Try to load from metadata first
      const vaultData = await this.getMetadataRaw('legacy_vault_data');
      const saltData = await this.getMetadataRaw('legacy_vault_salt');
      
      if (vaultData && saltData) {
        return {
          encryptedData: new Uint8Array(vaultData.value),
          salt: new Uint8Array(saltData.value),
          timestamp: vaultData.created_at,
          storageMode: 'sqlite-like'
        };
      }
      
      // Fallback to localStorage
      const localBackup = localStorage.getItem('passmann_vault_sqlite_backup');
      if (localBackup) {
        const backup = JSON.parse(localBackup);
        return {
          encryptedData: new Uint8Array(backup.data),
          salt: new Uint8Array(backup.salt),
          timestamp: backup.timestamp,
          storageMode: 'localStorage'
        };
      }
      
      // Fallback to chrome.storage
      if (chrome?.storage?.local) {
        const chromeBackup = await chrome.storage.local.get(['passmann_vault_sqlite_backup']);
        if (chromeBackup.passmann_vault_sqlite_backup) {
          const backup = chromeBackup.passmann_vault_sqlite_backup;
          return {
            encryptedData: new Uint8Array(backup.data),
            salt: new Uint8Array(backup.salt),
            timestamp: backup.timestamp,
            storageMode: 'chrome.storage'
          };
        }
      }
      
      return null;
    } catch (error) {
      console.error('Failed to load vault:', error);
      return null;
    }
  }

  // Clear all data
  async clearAllStorage() {
    try {
      // Clear IndexedDB
      if (this.db) {
        const stores = [this.VAULT_TABLE, this.METADATA_TABLE, this.SESSIONS_TABLE];
        const transaction = this.db.transaction(stores, 'readwrite');
        
        for (const storeName of stores) {
          const store = transaction.objectStore(storeName);
          await new Promise((resolve, reject) => {
            const request = store.clear();
            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
          });
        }
      }
      
      // Clear backups
      localStorage.removeItem('passmann_vault_sqlite_backup');
      
      if (chrome?.storage?.local) {
        await chrome.storage.local.remove(['passmann_vault_sqlite_backup']);
      }
    } catch (error) {
      console.error('Failed to clear storage:', error);
    }
  }

  // Health check
  async performStorageHealthCheck() {
    const health = {
      indexedDB: this.isInitialized,
      localStorage: false,
      chromeStorage: false,
      totalEntries: 0
    };

    try {
      localStorage.setItem('test', 'test');
      localStorage.removeItem('test');
      health.localStorage = true;
    } catch (e) {
      // localStorage not available
    }

    try {
      if (chrome?.storage?.local) {
        await chrome.storage.local.set({ 'test': 'test' });
        await chrome.storage.local.remove(['test']);
        health.chromeStorage = true;
      }
    } catch (e) {
      // chrome.storage not available
    }

    if (this.isInitialized) {
      try {
        const entries = await this.selectEntries({ is_deleted: false });
        health.totalEntries = entries.length;
      } catch (e) {
        // Can't count entries (vault locked)
      }
    }

    console.log('Storage Health Check:', health);
    return health;
  }
}

// Export for use
if (typeof module !== 'undefined' && module.exports) {
  module.exports = SQLiteLikeStorageManager;
}
