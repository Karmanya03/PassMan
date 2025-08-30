'use client';

import { useState, useEffect } from 'react';
import { 
  Eye, 
  EyeOff, 
  Copy, 
  Edit, 
  Trash2, 
  ExternalLink, 
  Star,
  Shield,
  AlertTriangle,
  CheckCircle
} from 'lucide-react';
import { Entry } from '@/types';
import { vaultService } from '@/lib/vault-service';
import { 
  formatDate, 
  getPasswordStrengthColor, 
  getPasswordStrengthText, 
  getCategoryColor,
  maskPassword,
  copyToClipboard
} from '@/lib/utils';
import toast from 'react-hot-toast';

interface EntryListProps {
  searchQuery?: string;
  onEditEntry: (entry: Entry) => void;
}

export default function EntryList({ searchQuery = '', onEditEntry }: EntryListProps) {
  const [entries, setEntries] = useState<Entry[]>([]);
  const [showPasswords, setShowPasswords] = useState<Record<string, boolean>>({});
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadEntries();
  }, []);

  useEffect(() => {
    if (searchQuery) {
      searchEntries();
    } else {
      loadEntries();
    }
  }, [searchQuery]);

  const loadEntries = async () => {
    try {
      const data = await vaultService.getEntries();
      setEntries(data);
    } catch (error) {
      toast.error('Failed to load entries');
    } finally {
      setIsLoading(false);
    }
  };

  const searchEntries = async () => {
    try {
      const data = await vaultService.searchEntries({
        query: searchQuery,
        case_sensitive: false,
        show_passwords: false
      });
      setEntries(data);
    } catch (error) {
      toast.error('Search failed');
    }
  };

  const togglePasswordVisibility = (entryId: string) => {
    setShowPasswords(prev => ({
      ...prev,
      [entryId]: !prev[entryId]
    }));
  };

  const handleCopy = async (text: string, type: string) => {
    try {
      await copyToClipboard(text);
      toast.success(`${type} copied to clipboard`);
    } catch (error) {
      toast.error('Failed to copy to clipboard');
    }
  };

  const handleDelete = async (entry: Entry) => {
    if (window.confirm(`Are you sure you want to delete the entry for ${entry.service}?`)) {
      try {
        await vaultService.deleteEntry(entry.id);
        await loadEntries();
        toast.success('Entry deleted successfully');
      } catch (error) {
        toast.error('Failed to delete entry');
      }
    }
  };

  const toggleFavorite = async (entry: Entry) => {
    try {
      await vaultService.updateEntry(entry.id, {
        is_favorite: !entry.is_favorite
      });
      await loadEntries();
      toast.success(entry.is_favorite ? 'Removed from favorites' : 'Added to favorites');
    } catch (error) {
      toast.error('Failed to update favorite status');
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-96">
        <div className="text-center space-y-4">
          <div className="animate-spin rounded-full h-16 w-16 border-4 border-blue-600 border-t-transparent mx-auto"></div>
          <p className="text-xl font-semibold text-gray-600">Loading your vault...</p>
          <p className="text-sm text-gray-500">Decrypting your secure passwords</p>
        </div>
      </div>
    );
  }

  if (entries.length === 0) {
    return (
      <div className="text-center py-20">
        <div className="w-24 h-24 bg-gradient-to-br from-blue-100 to-purple-100 rounded-3xl flex items-center justify-center mx-auto mb-8">
          <Shield className="w-12 h-12 text-blue-600" />
        </div>
        <h3 className="text-2xl font-bold text-gray-900 mb-4">
          {searchQuery ? 'No entries found' : 'Your vault is empty'}
        </h3>
        <p className="text-gray-600 mb-8 max-w-md mx-auto">
          {searchQuery 
            ? 'Try adjusting your search terms or check your spelling.' 
            : 'Start building your secure password collection by adding your first entry.'
          }
        </p>
        {!searchQuery && (
          <button
            onClick={() => onEditEntry({} as Entry)}
            className="btn-gradient text-white font-semibold py-3 px-8 rounded-xl shadow-lg hover:shadow-xl transition-all duration-300"
          >
            Add Your First Password
          </button>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {entries.map((entry) => (
        <div
          key={entry.id}
          className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow duration-200"
        >
          <div className="flex items-start justify-between">
            <div className="flex-1">
              {/* Header */}
              <div className="flex items-center mb-3">
                <div className="flex items-center space-x-3">
                  <h3 className="text-lg font-semibold text-gray-900">{entry.service}</h3>
                  <span className={`px-2 py-1 rounded-full text-xs font-medium ${getCategoryColor(entry.category)}`}>
                    {entry.category}
                  </span>
                  {entry.is_favorite && (
                    <Star className="w-4 h-4 text-yellow-500 fill-current" />
                  )}
                  {entry.two_factor_enabled && (
                    <div title="2FA Enabled">
                      <Shield className="w-4 h-4 text-green-500" />
                    </div>
                  )}
                </div>
                
                <button
                  onClick={() => toggleFavorite(entry)}
                  className="ml-auto text-gray-400 hover:text-yellow-500 transition-colors duration-200"
                  aria-label={entry.is_favorite ? "Remove from favorites" : "Add to favorites"}
                >
                  <Star className={`w-5 h-5 ${entry.is_favorite ? 'text-yellow-500 fill-current' : ''}`} />
                </button>
              </div>

              {/* Content */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
                <div>
                  <label className="block text-sm font-medium text-gray-500 mb-1">Username</label>
                  <div className="flex items-center space-x-2">
                    <span className="text-gray-900">{entry.username}</span>
                    <button
                      onClick={() => handleCopy(entry.username, 'Username')}
                      className="text-gray-400 hover:text-blue-600 transition-colors duration-200"
                      aria-label="Copy username"
                    >
                      <Copy className="w-4 h-4" />
                    </button>
                  </div>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-500 mb-1">Password</label>
                  <div className="flex items-center space-x-2">
                    <span className="font-mono text-gray-900">
                      {maskPassword(entry.password, showPasswords[entry.id])}
                    </span>
                    <button
                      onClick={() => togglePasswordVisibility(entry.id)}
                      className="text-gray-400 hover:text-blue-600 transition-colors duration-200"
                      aria-label={showPasswords[entry.id] ? "Hide password" : "Show password"}
                    >
                      {showPasswords[entry.id] ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                    </button>
                    <button
                      onClick={() => handleCopy(entry.password, 'Password')}
                      className="text-gray-400 hover:text-blue-600 transition-colors duration-200"
                      aria-label="Copy password"
                    >
                      <Copy className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              </div>

              {/* URL and Notes */}
              {entry.url && (
                <div className="mb-2">
                  <label className="block text-sm font-medium text-gray-500 mb-1">URL</label>
                  <div className="flex items-center space-x-2">
                    <a
                      href={entry.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 hover:text-blue-800 transition-colors duration-200"
                    >
                      {entry.url}
                    </a>
                    <ExternalLink className="w-4 h-4 text-gray-400" />
                  </div>
                </div>
              )}

              {entry.notes && (
                <div className="mb-4">
                  <label className="block text-sm font-medium text-gray-500 mb-1">Notes</label>
                  <p className="text-gray-700 text-sm">{entry.notes}</p>
                </div>
              )}

              {/* Tags */}
              {entry.tags.length > 0 && (
                <div className="mb-4">
                  <div className="flex flex-wrap gap-2">
                    {entry.tags.map((tag, index) => (
                      <span
                        key={index}
                        className="px-2 py-1 bg-gray-100 text-gray-700 text-xs rounded-full"
                      >
                        {tag}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {/* Password Strength */}
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center space-x-2">
                  <span className="text-sm text-gray-500">Password Strength:</span>
                  <span className={`text-sm font-medium ${getPasswordStrengthColor(entry.password_strength.score)}`}>
                    {getPasswordStrengthText(entry.password_strength.score)}
                  </span>
                  {entry.password_strength.score < 3 && (
                    <AlertTriangle className="w-4 h-4 text-orange-500" />
                  )}
                  {entry.password_strength.score >= 4 && (
                    <CheckCircle className="w-4 h-4 text-green-500" />
                  )}
                </div>
                
                <div className="text-sm text-gray-500">
                  Created: {formatDate(entry.created_at)}
                </div>
              </div>
            </div>

            {/* Actions */}
            <div className="flex items-center space-x-2 ml-4">
              <button
                onClick={() => onEditEntry(entry)}
                className="text-gray-400 hover:text-blue-600 transition-colors duration-200 p-2 rounded-lg hover:bg-blue-50"
                aria-label="Edit entry"
              >
                <Edit className="w-4 h-4" />
              </button>
              <button
                onClick={() => handleDelete(entry)}
                className="text-gray-400 hover:text-red-600 transition-colors duration-200 p-2 rounded-lg hover:bg-red-50"
                aria-label="Delete entry"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
