'use client';

import { useState } from 'react';
import { Lock, Shield, Eye, EyeOff } from 'lucide-react';
import { vaultService } from '@/lib/vault-service';
import toast from 'react-hot-toast';

interface UnlockVaultProps {
  onUnlock: () => void;
}

export default function UnlockVault({ onUnlock }: UnlockVaultProps) {
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  const handleUnlock = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!password.trim()) {
      toast.error('Please enter your master password');
      return;
    }

    setIsLoading(true);
    try {
      const success = await vaultService.unlock(password);
      if (success) {
        toast.success('Vault unlocked successfully!');
        onUnlock();
      } else {
        toast.error('Invalid master password');
      }
    } catch (error) {
      toast.error('Failed to unlock vault');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-gradient-mesh flex items-center justify-center p-4 relative overflow-hidden">
      {/* Animated Background Elements */}
      <div className="absolute inset-0 overflow-hidden">
        <div className="floating-shape absolute top-20 left-20 w-32 h-32 bg-blue-400/20 rounded-full blur-xl"></div>
        <div className="floating-shape animation-delay-2000 absolute top-40 right-32 w-24 h-24 bg-purple-400/20 rounded-full blur-xl"></div>
        <div className="floating-shape animation-delay-4000 absolute bottom-32 left-40 w-40 h-40 bg-pink-400/20 rounded-full blur-xl"></div>
        <div className="floating-shape animation-delay-1000 absolute bottom-20 right-20 w-28 h-28 bg-indigo-400/20 rounded-full blur-xl"></div>
      </div>

      <div className="glass-card p-10 w-full max-w-lg relative z-10">
        <div className="text-center mb-10">
          {/* Enhanced Logo */}
          <div className="relative mb-6">
            <div className="w-20 h-20 mx-auto bg-gradient-to-br from-blue-500 to-purple-600 rounded-2xl flex items-center justify-center shadow-2xl transform rotate-3 hover:rotate-0 transition-transform duration-300">
              <Shield className="w-10 h-10 text-white" />
            </div>
            <div className="absolute -top-1 -right-1 w-6 h-6 bg-green-500 rounded-full flex items-center justify-center">
              <div className="w-2 h-2 bg-white rounded-full"></div>
            </div>
          </div>
          
          <h1 className="text-4xl font-bold bg-gradient-to-r from-blue-600 via-purple-600 to-pink-600 bg-clip-text text-transparent mb-3">
            PassMan
          </h1>
          <p className="text-gray-600 text-lg">Your secure digital vault awaits</p>
          <p className="text-sm text-gray-500 mt-2">Enter your master password to unlock</p>
        </div>

        <form onSubmit={handleUnlock} className="space-y-8">
          <div className="form-group">
            <label htmlFor="password" className="form-label">
              Master Password
            </label>
            <div className="relative">
              <div className="absolute left-4 top-1/2 transform -translate-y-1/2 text-gray-400">
                <Lock className="w-5 h-5" />
              </div>
              <input
                type={showPassword ? 'text' : 'password'}
                id="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="form-input pl-12 pr-14 py-4 text-lg"
                placeholder="Enter your master password"
                disabled={isLoading}
                autoFocus
              />
              <button
                type="button"
                onClick={() => setShowPassword(!showPassword)}
                className="absolute right-4 top-1/2 transform -translate-y-1/2 p-2 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-all duration-200"
                disabled={isLoading}
                aria-label={showPassword ? "Hide password" : "Show password"}
              >
                {showPassword ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
              </button>
            </div>
          </div>

          <button
            type="submit"
            disabled={isLoading || !password.trim()}
            className="w-full btn-gradient text-white font-semibold py-4 px-6 rounded-xl shadow-xl hover:shadow-2xl disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-300 flex items-center justify-center text-lg relative overflow-hidden group"
          >
            <div className="absolute inset-0 bg-gradient-to-r from-white/0 via-white/20 to-white/0 translate-x-[-100%] group-hover:translate-x-[100%] transition-transform duration-700"></div>
            {isLoading ? (
              <div className="flex items-center">
                <div className="animate-spin rounded-full h-6 w-6 border-2 border-white border-t-transparent mr-3"></div>
                Unlocking...
              </div>
            ) : (
              <>
                <Lock className="w-6 h-6 mr-3" />
                Unlock Vault
              </>
            )}
          </button>
        </form>

        {/* Security Features */}
        <div className="mt-8 p-6 bg-gradient-to-r from-green-50 to-blue-50 rounded-xl border border-green-100">
          <h3 className="text-sm font-semibold text-gray-800 mb-3 flex items-center">
            <Shield className="w-4 h-4 mr-2 text-green-600" />
            Security Features
          </h3>
          <div className="grid grid-cols-2 gap-4 text-xs text-gray-600">
            <div className="flex items-center">
              <div className="w-2 h-2 bg-green-500 rounded-full mr-2"></div>
              XChaCha20Poly1305
            </div>
            <div className="flex items-center">
              <div className="w-2 h-2 bg-blue-500 rounded-full mr-2"></div>
              Argon2id Key Derivation
            </div>
            <div className="flex items-center">
              <div className="w-2 h-2 bg-purple-500 rounded-full mr-2"></div>
              Zero-Knowledge Architecture
            </div>
            <div className="flex items-center">
              <div className="w-2 h-2 bg-orange-500 rounded-full mr-2"></div>
              Client-Side Encryption
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="mt-6 text-center">
          <p className="text-xs text-gray-500">
            🛡️ Military-grade security you can trust
          </p>
        </div>
      </div>
    </div>
  );
}
