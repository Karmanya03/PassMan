'use client';

import { useState, useEffect } from 'react';
import { Save, X, Eye, EyeOff, RefreshCw, Star, Shield } from 'lucide-react';
import { Entry, EntryCategory, GeneratePasswordOptions } from '@/types';
import { vaultService } from '@/lib/vault-service';
import { getPasswordStrengthColor, getPasswordStrengthText } from '@/lib/utils';
import toast from 'react-hot-toast';

interface EntryFormProps {
  entry?: Entry;
  onSave: () => void;
  onCancel: () => void;
}

export default function EntryForm({ entry, onSave, onCancel }: EntryFormProps) {
  const [formData, setFormData] = useState({
    service: entry?.service || '',
    username: entry?.username || '',
    password: entry?.password || '',
    url: entry?.url || '',
    notes: entry?.notes || '',
    category: entry?.category || EntryCategory.PERSONAL,
    tags: entry?.tags?.join(', ') || '',
    is_favorite: entry?.is_favorite || false,
    two_factor_enabled: entry?.two_factor_enabled || false,
  });

  const [showPassword, setShowPassword] = useState(false);
  const [passwordStrength, setPasswordStrength] = useState({
    score: 0,
    feedback: [] as string[],
    crack_time_display: '',
  });
  const [isGenerating, setIsGenerating] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (formData.password) {
      calculatePasswordStrength(formData.password);
    }
  }, [formData.password]);

  const calculatePasswordStrength = (password: string) => {
    // Simple password strength calculation - in production you'd use zxcvbn
    let score = 0;
    const feedback: string[] = [];

    if (password.length >= 8) score++;
    if (password.length >= 12) score++;
    if (/[a-z]/.test(password) && /[A-Z]/.test(password)) score++;
    if (/\d/.test(password)) score++;
    if (/[!@#$%^&*()_+\-=\[\]{}|;:,.<>?]/.test(password)) score++;

    if (password.length < 8) feedback.push('Use at least 8 characters');
    if (!/[a-z]/.test(password)) feedback.push('Add lowercase letters');
    if (!/[A-Z]/.test(password)) feedback.push('Add uppercase letters');
    if (!/\d/.test(password)) feedback.push('Add numbers');
    if (!/[!@#$%^&*()_+\-=\[\]{}|;:,.<>?]/.test(password)) feedback.push('Add symbols');

    const crackTimes = ['instantly', 'seconds', 'minutes', 'hours', 'days', 'years'];
    
    setPasswordStrength({
      score,
      feedback,
      crack_time_display: crackTimes[Math.min(score, crackTimes.length - 1)],
    });
  };

  const generatePassword = async () => {
    setIsGenerating(true);
    try {
      const options: GeneratePasswordOptions = {
        length: 16,
        symbols: true,
        count: 1,
      };
      
      const passwords = await vaultService.generatePassword(options);
      if (passwords.length > 0) {
        setFormData(prev => ({ ...prev, password: passwords[0] }));
        toast.success('Password generated');
      }
    } catch (error) {
      toast.error('Failed to generate password');
    } finally {
      setIsGenerating(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!formData.service.trim() || !formData.username.trim() || !formData.password.trim()) {
      toast.error('Service, username, and password are required');
      return;
    }

    setIsSaving(true);
    try {
      const entryData = {
        service: formData.service.trim(),
        username: formData.username.trim(),
        password: formData.password,
        url: formData.url.trim() || undefined,
        notes: formData.notes.trim() || undefined,
        category: formData.category,
        tags: formData.tags.split(',').map(tag => tag.trim()).filter(tag => tag),
        custom_fields: {},
        password_strength: {
          score: passwordStrength.score,
          feedback: passwordStrength.feedback,
          warning: undefined,
          crack_time_display: passwordStrength.crack_time_display,
          entropy: passwordStrength.score * 15,
          has_common_passwords: false,
          has_dictionary_words: false,
          has_keyboard_patterns: false,
          has_repeated_patterns: false,
        },
        is_favorite: formData.is_favorite,
        two_factor_enabled: formData.two_factor_enabled,
      };

      if (entry) {
        await vaultService.updateEntry(entry.id, entryData);
        toast.success('Entry updated successfully');
      } else {
        await vaultService.addEntry(entryData);
        toast.success('Entry added successfully');
      }
      
      onSave();
    } catch (error) {
      toast.error('Failed to save entry');
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="glass-card p-8 max-w-4xl mx-auto">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h2 className="text-3xl font-bold bg-gradient-to-r from-blue-600 to-purple-600 bg-clip-text text-transparent">
            {entry ? 'Edit Entry' : 'Add New Entry'}
          </h2>
          <p className="text-gray-600 mt-2">
            {entry ? 'Update your password entry details' : 'Securely store a new password in your vault'}
          </p>
        </div>
        <button
          onClick={onCancel}
          className="p-3 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-xl transition-all duration-200"
          aria-label="Close form"
        >
          <X className="w-6 h-6" />
        </button>
      </div>

      <form onSubmit={handleSubmit} className="space-y-8">
        {/* Basic Information Section */}
        <div className="bg-gradient-to-r from-blue-50 to-purple-50 rounded-2xl p-6 border border-blue-100">
          <h3 className="text-lg font-semibold text-gray-900 mb-4 flex items-center">
            <Shield className="w-5 h-5 mr-2 text-blue-600" />
            Basic Information
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="form-group">
              <label htmlFor="service" className="form-label">
                Service Name *
              </label>
              <input
                type="text"
                id="service"
                value={formData.service}
                onChange={(e) => setFormData(prev => ({ ...prev, service: e.target.value }))}
                className="form-input"
                placeholder="e.g., Gmail, GitHub, Netflix"
                required
              />
            </div>

            <div className="form-group">
              <label htmlFor="username" className="form-label">
                Username/Email *
              </label>
              <input
                type="text"
                id="username"
                value={formData.username}
                onChange={(e) => setFormData(prev => ({ ...prev, username: e.target.value }))}
                className="form-input"
                placeholder="username or email@example.com"
                required
              />
            </div>
          </div>
        </div>

        {/* Password Section */}
        <div className="bg-gradient-to-r from-green-50 to-blue-50 rounded-2xl p-6 border border-green-100">
          <h3 className="text-lg font-semibold text-gray-900 mb-4 flex items-center">
            <Eye className="w-5 h-5 mr-2 text-green-600" />
            Password & Security
          </h3>
          <div className="form-group">
            <label htmlFor="password" className="form-label">
              Password *
            </label>
            <div className="relative">
              <input
                type={showPassword ? 'text' : 'password'}
                id="password"
                value={formData.password}
                onChange={(e) => setFormData(prev => ({ ...prev, password: e.target.value }))}
                className="form-input pr-24"
                placeholder="Enter or generate a secure password"
                required
              />
              <div className="absolute right-3 top-1/2 transform -translate-y-1/2 flex items-center space-x-2">
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="p-2 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-all duration-200"
                  aria-label={showPassword ? "Hide password" : "Show password"}
                >
                  {showPassword ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
                <button
                  type="button"
                  onClick={generatePassword}
                  disabled={isGenerating}
                  className="p-2 text-gray-400 hover:text-blue-600 hover:bg-blue-50 rounded-lg transition-all duration-200 disabled:opacity-50"
                  aria-label="Generate password"
                >
                  <RefreshCw className={`w-4 h-4 ${isGenerating ? 'animate-spin' : ''}`} />
                </button>
              </div>
            </div>
            
            {/* Enhanced Password Strength Indicator */}
            {formData.password && (
              <div className="mt-4 p-4 bg-white rounded-xl border border-gray-200">
                <div className="flex items-center justify-between mb-3">
                  <span className="text-sm font-medium text-gray-700">Password Strength</span>
                  <span className={`text-sm font-bold px-3 py-1 rounded-full ${
                    passwordStrength.score <= 1 ? 'bg-red-100 text-red-700' :
                    passwordStrength.score <= 2 ? 'bg-orange-100 text-orange-700' :
                    passwordStrength.score <= 3 ? 'bg-yellow-100 text-yellow-700' :
                    passwordStrength.score <= 4 ? 'bg-green-100 text-green-700' : 'bg-emerald-100 text-emerald-700'
                  }`}>
                    {getPasswordStrengthText(passwordStrength.score)}
                  </span>
                </div>
                <div className="w-full bg-gray-200 rounded-full h-3 mb-3 overflow-hidden">
                  <div
                    className={`h-3 rounded-full transition-all duration-500 ${
                      passwordStrength.score <= 1 ? 'bg-red-500 w-1/5' :
                      passwordStrength.score <= 2 ? 'bg-orange-500 w-2/5' :
                      passwordStrength.score <= 3 ? 'bg-yellow-500 w-3/5' :
                      passwordStrength.score <= 4 ? 'bg-green-500 w-4/5' : 'bg-emerald-500 w-full'
                    }`}
                  ></div>
                </div>
                {passwordStrength.feedback.length > 0 && (
                  <div className="text-xs text-gray-600 bg-gray-50 rounded-lg p-2">
                    <span className="font-medium">Suggestions:</span> {passwordStrength.feedback.join(', ')}
                  </div>
                )}
                <div className="text-xs text-gray-500 mt-2">
                  Estimated crack time: <span className="font-medium">{passwordStrength.crack_time_display}</span>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Additional Details Section */}
        <div className="bg-gradient-to-r from-purple-50 to-pink-50 rounded-2xl p-6 border border-purple-100">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">Additional Details</h3>
          <div className="space-y-6">
            {/* URL */}
            <div className="form-group">
              <label htmlFor="url" className="form-label">
                Website URL
              </label>
              <input
                type="url"
                id="url"
                value={formData.url}
                onChange={(e) => setFormData(prev => ({ ...prev, url: e.target.value }))}
                className="form-input"
                placeholder="https://example.com"
              />
            </div>

            {/* Category and Toggles */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              <div className="form-group">
                <label htmlFor="category" className="form-label">
                  Category
                </label>
                <select
                  id="category"
                  value={formData.category}
                  onChange={(e) => setFormData(prev => ({ ...prev, category: e.target.value as EntryCategory }))}
                  className="form-input"
                >
                  {Object.values(EntryCategory).map(category => (
                    <option key={category} value={category}>
                      {category}
                    </option>
                  ))}
                </select>
              </div>

              <div className="flex flex-col space-y-4 justify-center">
                <label className="flex items-center p-3 bg-white rounded-xl border border-gray-200 hover:border-blue-300 transition-colors cursor-pointer">
                  <input
                    type="checkbox"
                    checked={formData.is_favorite}
                    onChange={(e) => setFormData(prev => ({ ...prev, is_favorite: e.target.checked }))}
                    className="rounded border-gray-300 text-yellow-600 focus:ring-yellow-500 mr-3"
                  />
                  <Star className={`w-4 h-4 mr-2 ${formData.is_favorite ? 'text-yellow-500 fill-current' : 'text-gray-400'}`} />
                  <span className="text-sm font-medium text-gray-700">Mark as favorite</span>
                </label>

                <label className="flex items-center p-3 bg-white rounded-xl border border-gray-200 hover:border-green-300 transition-colors cursor-pointer">
                  <input
                    type="checkbox"
                    checked={formData.two_factor_enabled}
                    onChange={(e) => setFormData(prev => ({ ...prev, two_factor_enabled: e.target.checked }))}
                    className="rounded border-gray-300 text-green-600 focus:ring-green-500 mr-3"
                  />
                  <Shield className={`w-4 h-4 mr-2 ${formData.two_factor_enabled ? 'text-green-500' : 'text-gray-400'}`} />
                  <span className="text-sm font-medium text-gray-700">Two-factor authentication</span>
                </label>
              </div>
            </div>

            {/* Tags */}
            <div className="form-group">
              <label htmlFor="tags" className="form-label">
                Tags
              </label>
              <input
                type="text"
                id="tags"
                value={formData.tags}
                onChange={(e) => setFormData(prev => ({ ...prev, tags: e.target.value }))}
                className="form-input"
                placeholder="work, important, social (comma-separated)"
              />
            </div>

            {/* Notes */}
            <div className="form-group">
              <label htmlFor="notes" className="form-label">
                Notes
              </label>
              <textarea
                id="notes"
                value={formData.notes}
                onChange={(e) => setFormData(prev => ({ ...prev, notes: e.target.value }))}
                rows={4}
                className="form-input resize-none"
                placeholder="Additional notes, recovery codes, or important information..."
              />
            </div>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center justify-end space-x-4 pt-6">
          <button
            type="button"
            onClick={onCancel}
            className="px-8 py-3 text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-xl font-medium transition-all duration-200 hover:shadow-md"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isSaving}
            className="btn-gradient text-white font-semibold px-8 py-3 rounded-xl shadow-lg hover:shadow-xl disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-300 flex items-center"
          >
            {isSaving ? (
              <div className="animate-spin rounded-full h-5 w-5 border-2 border-white border-t-transparent mr-2"></div>
            ) : (
              <Save className="w-5 h-5 mr-2" />
            )}
            {entry ? 'Update Entry' : 'Add Entry'}
          </button>
        </div>
      </form>
    </div>
  );
}
