import { Entry, VaultStats, AuditLogEntry, GeneratePasswordOptions, SearchOptions } from '@/types';

// Mock data for development - in production this would connect to the Rust backend
class VaultService {
  private entries: Entry[] = [];
  private isUnlocked = false;

  async unlock(masterPassword: string): Promise<boolean> {
    // In production, this would validate against the Rust backend
    if (masterPassword.length >= 8) {
      this.isUnlocked = true;
      // Load demo entries
      this.loadDemoEntries();
      return true;
    }
    return false;
  }

  async lock(): Promise<void> {
    this.isUnlocked = false;
    this.entries = [];
  }

  async addEntry(entry: Omit<Entry, 'id' | 'created_at' | 'modified_at' | 'access_count' | 'version'>): Promise<Entry> {
    if (!this.isUnlocked) throw new Error('Vault is locked');
    
    const newEntry: Entry = {
      ...entry,
      id: crypto.randomUUID(),
      created_at: new Date().toISOString(),
      modified_at: new Date().toISOString(),
      access_count: 0,
      version: 1,
    };

    this.entries.push(newEntry);
    return newEntry;
  }

  async getEntries(): Promise<Entry[]> {
    if (!this.isUnlocked) throw new Error('Vault is locked');
    return this.entries;
  }

  async searchEntries(options: SearchOptions): Promise<Entry[]> {
    if (!this.isUnlocked) throw new Error('Vault is locked');
    
    const query = options.case_sensitive ? options.query : options.query.toLowerCase();
    
    return this.entries.filter(entry => {
      const service = options.case_sensitive ? entry.service : entry.service.toLowerCase();
      const username = options.case_sensitive ? entry.username : entry.username.toLowerCase();
      
      return service.includes(query) || username.includes(query) || 
             entry.tags.some(tag => 
               options.case_sensitive ? tag.includes(query) : tag.toLowerCase().includes(query)
             );
    });
  }

  async deleteEntry(id: string): Promise<boolean> {
    if (!this.isUnlocked) throw new Error('Vault is locked');
    
    const index = this.entries.findIndex(entry => entry.id === id);
    if (index === -1) return false;
    
    this.entries.splice(index, 1);
    return true;
  }

  async updateEntry(id: string, updates: Partial<Entry>): Promise<Entry | null> {
    if (!this.isUnlocked) throw new Error('Vault is locked');
    
    const entry = this.entries.find(e => e.id === id);
    if (!entry) return null;
    
    Object.assign(entry, updates, {
      modified_at: new Date().toISOString(),
      version: entry.version + 1
    });
    
    return entry;
  }

  async generatePassword(options: GeneratePasswordOptions): Promise<string[]> {
    const passwords: string[] = [];
    
    for (let i = 0; i < options.count; i++) {
      let charset = '';
      
      if (options.includeLowercase) charset += 'abcdefghijklmnopqrstuvwxyz';
      if (options.includeUppercase) charset += 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
      if (options.includeNumbers) charset += '0123456789';
      if (options.includeSymbols) charset += '!@#$%^&*()_+-=[]{}|;:,.<>?';
      
      if (options.excludeSimilar) {
        charset = charset.replace(/[il1Lo0O]/g, '');
      }
      
      // Ensure at least one character type is selected
      if (!charset) {
        charset = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
      }
      
      let password = '';
      for (let j = 0; j < options.length; j++) {
        password += charset.charAt(Math.floor(Math.random() * charset.length));
      }
      passwords.push(password);
    }
    
    return passwords;
  }

  async getVaultStats(): Promise<VaultStats> {
    if (!this.isUnlocked) throw new Error('Vault is locked');
    
    const categoryCounts: Record<string, number> = {};
    let weakPasswords = 0;
    let duplicatePasswords = 0;
    let twoFactorEnabled = 0;

    this.entries.forEach(entry => {
      categoryCounts[entry.category] = (categoryCounts[entry.category] || 0) + 1;
      if (entry.password_strength.score < 3) weakPasswords++;
      if (entry.two_factor_enabled) twoFactorEnabled++;
    });

    // Check for duplicate passwords
    const passwordCounts = new Map<string, number>();
    this.entries.forEach(entry => {
      passwordCounts.set(entry.password, (passwordCounts.get(entry.password) || 0) + 1);
    });
    duplicatePasswords = Array.from(passwordCounts.values()).filter(count => count > 1).length;

    return {
      total_entries: this.entries.length,
      categories_count: categoryCounts,
      weak_passwords: weakPasswords,
      duplicate_passwords: duplicatePasswords,
      old_passwords: 0, // Would need to calculate based on modified_at
      breached_passwords: 0, // Would need breach checking service
      two_factor_enabled: twoFactorEnabled,
      last_backup: new Date().toISOString(),
      vault_size: `${Math.round(JSON.stringify(this.entries).length / 1024)}KB`,
      security_score: Math.max(1, 10 - Math.floor((weakPasswords + duplicatePasswords) / this.entries.length * 10))
    };
  }

  async getAuditLogs(): Promise<AuditLogEntry[]> {
    // Mock audit logs - in production would come from Rust backend
    return [
      {
        timestamp: new Date().toISOString(),
        action: "VAULT_UNLOCKED",
        details: "Vault successfully unlocked"
      },
      {
        timestamp: new Date(Date.now() - 3600000).toISOString(),
        action: "ENTRY_ADDED",
        details: "New entry added for github.com"
      }
    ];
  }

  private loadDemoEntries(): void {
    this.entries = [
      {
        id: '1',
        service: 'Gmail',
        username: 'user@gmail.com',
        password: 'SecurePass123!',
        url: 'https://gmail.com',
        notes: 'Primary email account',
        category: 'PERSONAL' as any,
        tags: ['email', 'primary'],
        custom_fields: {},
        password_strength: {
          score: 4,
          feedback: ['Strong password'],
          crack_time_display: '10+ years',
          entropy: 65.5,
          has_common_passwords: false,
          has_dictionary_words: false,
          has_keyboard_patterns: false,
          has_repeated_patterns: false
        },
        created_at: new Date().toISOString(),
        modified_at: new Date().toISOString(),
        access_count: 5,
        is_favorite: true,
        version: 1,
        two_factor_enabled: true
      },
      {
        id: '2',
        service: 'GitHub',
        username: 'developer',
        password: 'CodeMaster2024#',
        url: 'https://github.com',
        notes: 'Development platform',
        category: 'WORK' as any,
        tags: ['development', 'git'],
        custom_fields: { 'API Token': 'ghp_xxxxxxxxxxxx' },
        password_strength: {
          score: 5,
          feedback: ['Excellent password'],
          crack_time_display: '100+ years',
          entropy: 72.1,
          has_common_passwords: false,
          has_dictionary_words: false,
          has_keyboard_patterns: false,
          has_repeated_patterns: false
        },
        created_at: new Date(Date.now() - 86400000).toISOString(),
        modified_at: new Date(Date.now() - 86400000).toISOString(),
        access_count: 12,
        is_favorite: true,
        version: 1,
        two_factor_enabled: true
      }
    ];
  }

  isVaultUnlocked(): boolean {
    return this.isUnlocked;
  }
}

export const vaultService = new VaultService();
