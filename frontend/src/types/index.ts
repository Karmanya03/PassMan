// Authentication types
export interface User {
  id: string;
  username: string;
  email: string;
}

export interface AuthState {
  isAuthenticated: boolean;
  user: User | null;
  token: string | null;
}

export interface RegisterRequest {
  username: string;
  email: string;
  master_password: string;
  confirm_password: string;
}

export interface LoginRequest {
  username: string;
  master_password: string;
}

export interface AuthResponse {
  token: string;
  user_id: string;
  username: string;
}

// Entry and vault types
export interface Entry {
  id: string;
  service: string;
  username: string;
  password: string;
  url?: string;
  notes?: string;
  category: EntryCategory;
  tags: string[];
  custom_fields: Record<string, string>;
  password_strength: PasswordStrengthInfo;
  created_at: string;
  modified_at: string;
  last_accessed?: string;
  access_count: number;
  is_favorite: boolean;
  expiry_date?: string;
  version: number;
  breach_status?: BreachStatus;
  two_factor_enabled?: boolean;
}

export enum EntryCategory {
  PERSONAL = "Personal",
  WORK = "Work", 
  FINANCIAL = "Financial",
  SOCIAL = "Social",
  GAMING = "Gaming",
  SHOPPING = "Shopping",
  OTHER = "Other"
}

export interface PasswordStrengthInfo {
  score: number;
  feedback: string[];
  warning?: string;
  crack_time_display: string;
  entropy: number;
  has_common_passwords: boolean;
  has_dictionary_words: boolean;
  has_keyboard_patterns: boolean;
  has_repeated_patterns: boolean;
}

export interface BreachStatus {
  is_breached: boolean;
  breach_count: number;
  last_breach_date?: string;
  breach_details: string[];
}

export interface VaultStats {
  total_entries: number;
  categories_count: Record<string, number>;
  weak_passwords: number;
  duplicate_passwords: number;
  old_passwords: number;
  breached_passwords: number;
  two_factor_enabled: number;
  last_backup: string;
  vault_size: string;
  security_score: number;
}

export interface AuditLogEntry {
  timestamp: string;
  action: string;
  details: string;
  ip_address?: string;
  user_agent?: string;
}

export interface GeneratePasswordOptions {
  length: number;
  includeUppercase: boolean;
  includeLowercase: boolean;
  includeNumbers: boolean;
  includeSymbols: boolean;
  excludeSimilar: boolean;
  count: number;
}

export interface SearchOptions {
  query: string;
  case_sensitive: boolean;
  show_passwords: boolean;
}

export interface ExportOptions {
  format: 'json' | 'csv';
}

export interface ImportOptions {
  format: 'json' | 'csv';
  force: boolean;
}
