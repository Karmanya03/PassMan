'use client';

import { useState } from 'react';
import { RefreshCw, Copy, Check, Key, Settings, Lock, Shield, Sliders, Type } from 'lucide-react';
import { vaultService } from '@/lib/vault-service';
import { copyToClipboard, cn } from '@/lib/utils';
import toast from 'react-hot-toast';

export default function PasswordGenerator() {
  const [options, setOptions] = useState({
    length: 16,
    includeUppercase: true,
    includeLowercase: true,
    includeNumbers: true,
    includeSymbols: true,
    excludeSimilar: false,
    count: 1,
  });

  const [generatedPasswords, setGeneratedPasswords] = useState<string[]>([]);
  const [isGenerating, setIsGenerating] = useState(false);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

  const generatePasswords = async () => {
    setIsGenerating(true);
    try {
      const passwords = await vaultService.generatePassword(options);
      setGeneratedPasswords(passwords);
      toast.success(`Generated ${passwords.length} password${passwords.length > 1 ? 's' : ''}`);
    } catch (error) {
      toast.error('Failed to generate passwords');
    } finally {
      setIsGenerating(false);
    }
  };

  const handleCopy = async (password: string, index: number) => {
    try {
      await copyToClipboard(password);
      setCopiedIndex(index);
      toast.success('Password copied to clipboard');
      
      setTimeout(() => setCopiedIndex(null), 2000);
    } catch (error) {
      toast.error('Failed to copy password');
    }
  };

  const getPasswordStrength = (password: string) => {
    let score = 0;
    if (password.length >= 8) score++;
    if (password.length >= 12) score++;
    if (/[a-z]/.test(password) && /[A-Z]/.test(password)) score++;
    if (/\d/.test(password)) score++;
    if (/[!@#$%^&*()_+\-=\[\]{}|;:,.<>?]/.test(password)) score++;

    const levels = [
      { text: 'Very Weak', color: 'text-red-500', bg: 'bg-red-500' },
      { text: 'Weak', color: 'text-orange-500', bg: 'bg-orange-500' },
      { text: 'Fair', color: 'text-yellow-500', bg: 'bg-yellow-500' },
      { text: 'Good', color: 'text-blue-500', bg: 'bg-blue-500' },
      { text: 'Strong', color: 'text-green-500', bg: 'bg-green-500' },
      { text: 'Very Strong', color: 'text-emerald-500', bg: 'bg-emerald-500' },
    ];
    
    return { score, ...levels[Math.min(score, 5)] };
  };

  return (
    <div className="max-w-6xl mx-auto space-y-8">
      {/* Header */}
      <div className="text-center mb-12">
        <div className="inline-flex items-center justify-center w-16 h-16 bg-gradient-to-br from-purple-600 to-blue-600 rounded-3xl shadow-2xl mb-6 relative">
          <Key className="w-8 h-8 text-white" />
          <div className="absolute -top-1 -right-1 w-5 h-5 bg-gradient-to-r from-green-400 to-emerald-500 rounded-full border-2 border-white flex items-center justify-center">
            <Sliders className="w-2.5 h-2.5 text-white" />
          </div>
        </div>
        <h1 className="text-4xl font-bold gradient-text mb-4">Password Generator</h1>
        <p className="text-xl text-gray-600 max-w-2xl mx-auto">
          Create cryptographically secure passwords with advanced customization options
        </p>
      </div>

      {/* Generator Controls */}
      <div className="card-elegant p-8 shadow-2xl">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          {/* Left Column - Basic Options */}
          <div className="space-y-6">
            <h3 className="text-xl font-semibold text-gray-900 flex items-center">
              <Settings className="w-5 h-5 mr-3 text-purple-600" />
              Basic Options
            </h3>
            
            {/* Length Control */}
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <label className="text-sm font-semibold text-gray-700">Password Length</label>
                <div className="bg-gradient-to-r from-purple-600 to-blue-600 text-white px-4 py-2 rounded-xl font-bold text-lg min-w-16 text-center">
                  {options.length}
                </div>
              </div>
              <input
                type="range"
                min="4"
                max="128"
                value={options.length}
                onChange={(e) => setOptions(prev => ({ ...prev, length: parseInt(e.target.value) }))}
                className="w-full h-3 bg-gray-200 rounded-lg appearance-none cursor-pointer range-lg"
                aria-label="Password length"
              />
              <div className="flex justify-between text-xs text-gray-500 font-medium">
                <span>Minimum (4)</span>
                <span>Maximum (128)</span>
              </div>
            </div>

            {/* Count Control */}
            <div className="space-y-3">
              <label className="text-sm font-semibold text-gray-700">Number of Passwords</label>
              <div className="grid grid-cols-4 gap-2">
                {[1, 3, 5, 10].map(num => (
                  <button
                    key={num}
                    onClick={() => setOptions(prev => ({ ...prev, count: num }))}
                    className={cn(
                      "py-3 px-4 rounded-xl font-semibold text-sm transition-all duration-200",
                      options.count === num
                        ? "bg-gradient-to-r from-purple-600 to-blue-600 text-white shadow-lg transform scale-105"
                        : "bg-gray-100 text-gray-700 hover:bg-gray-200 hover:transform hover:scale-105"
                    )}
                  >
                    {num}
                  </button>
                ))}
              </div>
            </div>
          </div>

          {/* Right Column - Character Types */}
          <div className="space-y-6">
            <h3 className="text-xl font-semibold text-gray-900 flex items-center">
              <Type className="w-5 h-5 mr-3 text-blue-600" />
              Character Types
            </h3>
            
            <div className="grid grid-cols-1 gap-4">
              {[
                { key: 'includeUppercase', label: 'Uppercase Letters', example: 'A-Z', color: 'blue' },
                { key: 'includeLowercase', label: 'Lowercase Letters', example: 'a-z', color: 'green' },
                { key: 'includeNumbers', label: 'Numbers', example: '0-9', color: 'yellow' },
                { key: 'includeSymbols', label: 'Special Symbols', example: '!@#$', color: 'purple' },
                { key: 'excludeSimilar', label: 'Exclude Similar Characters', example: 'il1Lo0O', color: 'gray', exclude: true }
              ].map((option) => (
                <label
                  key={option.key}
                  className={cn(
                    "flex items-center justify-between p-4 rounded-xl border-2 cursor-pointer transition-all duration-200",
                    options[option.key as keyof typeof options]
                      ? `border-${option.color}-300 bg-${option.color}-50`
                      : "border-gray-200 hover:border-gray-300 hover:bg-gray-50"
                  )}
                >
                  <div className="flex items-center space-x-4">
                    <input
                      type="checkbox"
                      checked={options[option.key as keyof typeof options] as boolean}
                      onChange={(e) => setOptions(prev => ({ ...prev, [option.key]: e.target.checked }))}
                      className={`w-5 h-5 rounded border-2 text-${option.color}-600 focus:ring-${option.color}-500`}
                    />
                    <div>
                      <div className="font-semibold text-gray-900">{option.label}</div>
                      <div className="text-sm text-gray-500 font-mono">{option.example}</div>
                    </div>
                  </div>
                  {options[option.key as keyof typeof options] && (
                    <Check className={`w-5 h-5 text-${option.color}-600`} />
                  )}
                </label>
              ))}
            </div>
          </div>
        </div>

        {/* Generate Button */}
        <div className="mt-10 flex justify-center">
          <button
            onClick={generatePasswords}
            disabled={isGenerating || (!options.includeUppercase && !options.includeLowercase && !options.includeNumbers && !options.includeSymbols)}
            className="btn-gradient text-white font-bold py-4 px-12 rounded-2xl shadow-2xl hover:shadow-3xl disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-300 flex items-center text-lg"
          >
            {isGenerating ? (
              <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-white mr-3"></div>
            ) : (
              <RefreshCw className="w-6 h-6 mr-3" />
            )}
            Generate Password{options.count > 1 ? 's' : ''}
          </button>
        </div>
      </div>

      {/* Generated Passwords */}
      {generatedPasswords.length > 0 && (
        <div className="card-elegant p-8 shadow-2xl animate-slide-up">
          <div className="flex items-center justify-between mb-8">
            <h2 className="text-2xl font-bold text-gray-900 flex items-center">
              <Lock className="w-6 h-6 mr-3 text-green-600" />
              Generated Passwords
            </h2>
            <div className="bg-green-100 text-green-800 px-4 py-2 rounded-xl font-semibold text-sm">
              {generatedPasswords.length} password{generatedPasswords.length > 1 ? 's' : ''} created
            </div>
          </div>
          
          <div className="grid gap-6">
            {generatedPasswords.map((password, index) => {
              const strength = getPasswordStrength(password);
              
              return (
                <div key={index} className="bg-gradient-to-r from-gray-50 to-blue-50 border border-gray-200 rounded-2xl p-6 hover:shadow-lg transition-all duration-300">
                  <div className="flex items-center justify-between mb-4">
                    <div className="flex items-center space-x-3">
                      <div className="w-8 h-8 bg-gradient-to-r from-blue-600 to-purple-600 rounded-xl flex items-center justify-center">
                        <span className="text-white font-bold text-sm">{index + 1}</span>
                      </div>
                      <span className="font-mono text-xl text-gray-900 break-all tracking-wide">
                        {password}
                      </span>
                    </div>
                    <button
                      onClick={() => handleCopy(password, index)}
                      className={cn(
                        "flex items-center px-6 py-3 rounded-xl font-semibold text-sm transition-all duration-200 shadow-md hover:shadow-lg transform hover:scale-105",
                        copiedIndex === index
                          ? 'bg-gradient-to-r from-green-500 to-emerald-500 text-white'
                          : 'bg-white text-gray-700 hover:bg-gray-50 border border-gray-200'
                      )}
                    >
                      {copiedIndex === index ? (
                        <>
                          <Check className="w-4 h-4 mr-2" />
                          Copied!
                        </>
                      ) : (
                        <>
                          <Copy className="w-4 h-4 mr-2" />
                          Copy
                        </>
                      )}
                    </button>
                  </div>
                  
                  {/* Strength Indicator */}
                  <div className="space-y-3">
                    <div className="flex items-center justify-between">
                      <span className="text-sm font-semibold text-gray-700">Security Strength</span>
                      <span className={cn(
                        "text-sm font-bold px-3 py-1 rounded-full",
                        strength.score <= 1 ? 'bg-red-100 text-red-700' :
                        strength.score <= 2 ? 'bg-orange-100 text-orange-700' :
                        strength.score <= 3 ? 'bg-yellow-100 text-yellow-700' :
                        strength.score <= 4 ? 'bg-blue-100 text-blue-700' : 'bg-green-100 text-green-700'
                      )}>
                        {strength.text}
                      </span>
                    </div>
                    <div className="w-full bg-gray-200 rounded-full h-3 overflow-hidden">
                      <div
                        className={cn(
                          "h-3 rounded-full transition-all duration-700 relative",
                          strength.bg,
                          strength.score <= 1 ? 'w-1/5' :
                          strength.score <= 2 ? 'w-2/5' :
                          strength.score <= 3 ? 'w-3/5' :
                          strength.score <= 4 ? 'w-4/5' : 'w-full'
                        )}
                      >
                        <div className="absolute inset-0 bg-gradient-to-r from-transparent to-white opacity-30 animate-pulse"></div>
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Security Tips */}
      <div className="bg-gradient-to-br from-blue-50 to-indigo-100 border border-blue-200 rounded-2xl p-8 shadow-lg">
        <div className="flex items-start space-x-4">
          <div className="w-12 h-12 bg-gradient-to-br from-blue-600 to-indigo-600 rounded-2xl flex items-center justify-center shadow-lg">
            <Shield className="w-6 h-6 text-white" />
          </div>
          <div className="flex-1">
            <h3 className="text-xl font-bold text-blue-900 mb-4">Password Security Best Practices</h3>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {[
                'Use at least 12-16 characters for optimal security',
                'Include a mix of all character types',
                'Avoid using personal information or common words',
                'Use unique passwords for each account',
                'Consider using a passphrase for easier memorization',
                'Enable two-factor authentication when available'
              ].map((tip, index) => (
                <div key={index} className="flex items-start space-x-3">
                  <div className="w-6 h-6 bg-blue-600 rounded-full flex items-center justify-center flex-shrink-0 mt-0.5">
                    <span className="text-white text-xs font-bold">{index + 1}</span>
                  </div>
                  <span className="text-blue-800 font-medium">{tip}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
