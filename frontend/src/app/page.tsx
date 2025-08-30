'use client';

import { useState, useEffect } from 'react';
import { Toaster } from 'react-hot-toast';
import AuthFlow from '@/components/AuthFlow';
import DashboardLayout from '@/components/DashboardLayout';
import EntryList from '@/components/EntryList';
import EntryForm from '@/components/EntryForm';
import PasswordGenerator from '@/components/PasswordGenerator';
import { Entry, AuthState, User } from '@/types';
import { vaultService } from '@/lib/vault-service';

export default function Home() {
  const [authState, setAuthState] = useState<AuthState>({
    isAuthenticated: false,
    user: null,
    token: null,
  });
  const [currentView, setCurrentView] = useState('vault');
  const [editingEntry, setEditingEntry] = useState<Entry | undefined>(undefined);
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    // Check for existing session
    const token = localStorage.getItem('passman_token');
    const userData = localStorage.getItem('passman_user');
    
    if (token && userData) {
      try {
        const user = JSON.parse(userData);
        setAuthState({
          isAuthenticated: true,
          user,
          token,
        });
        // Initialize vault with mock data
        vaultService.unlock('mock-password');
      } catch (error) {
        // Clear invalid data
        localStorage.removeItem('passman_token');
        localStorage.removeItem('passman_user');
      }
    }
  }, []);

  const handleAuthenticated = (token: string, user: User) => {
    const authData = {
      isAuthenticated: true,
      user,
      token,
    };
    
    setAuthState(authData);
    
    // Store session
    localStorage.setItem('passman_token', token);
    localStorage.setItem('passman_user', JSON.stringify(user));
    
    // Initialize vault
    vaultService.unlock('mock-password');
  };

  const handleLogout = () => {
    setAuthState({
      isAuthenticated: false,
      user: null,
      token: null,
    });
    
    // Clear session
    localStorage.removeItem('passman_token');
    localStorage.removeItem('passman_user');
    
    // Lock vault
    vaultService.lock();
    
    // Reset view
    setCurrentView('vault');
    setEditingEntry(undefined);
  };

  const handleViewChange = (view: string) => {
    setCurrentView(view);
    if (view !== 'add-entry') {
      setEditingEntry(undefined);
    }
  };

  const handleEditEntry = (entry: Entry) => {
    setEditingEntry(entry);
    setCurrentView('add-entry');
  };

  const handleSaveEntry = () => {
    setEditingEntry(undefined);
    setCurrentView('vault');
  };

  const handleCancelEdit = () => {
    setEditingEntry(undefined);
    setCurrentView('vault');
  };

  const renderContent = () => {
    switch (currentView) {
      case 'vault':
        return (
          <div className="p-4 sm:p-6">
            <EntryList 
              searchQuery={searchQuery}
              onEditEntry={handleEditEntry}
            />
          </div>
        );
      
      case 'add-entry':
        return (
          <div className="p-4 sm:p-6">
            <EntryForm
              entry={editingEntry}
              onSave={handleSaveEntry}
              onCancel={handleCancelEdit}
            />
          </div>
        );
      
      case 'generate':
        return (
          <div className="p-4 sm:p-6">
            <PasswordGenerator />
          </div>
        );
      
      case 'analytics':
        return (
          <div className="p-4 sm:p-6">
            <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
              <h2 className="text-2xl font-bold text-gray-900 mb-4">Vault Analytics</h2>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                <div className="bg-blue-50 rounded-lg p-4">
                  <h3 className="text-lg font-semibold text-blue-900">Security Score</h3>
                  <p className="text-3xl font-bold text-blue-600">8.5/10</p>
                </div>
                <div className="bg-green-50 rounded-lg p-4">
                  <h3 className="text-lg font-semibold text-green-900">Strong Passwords</h3>
                  <p className="text-3xl font-bold text-green-600">85%</p>
                </div>
                <div className="bg-yellow-50 rounded-lg p-4">
                  <h3 className="text-lg font-semibold text-yellow-900">Weak Passwords</h3>
                  <p className="text-3xl font-bold text-yellow-600">2</p>
                </div>
              </div>
            </div>
          </div>
        );
      
      case 'audit':
        return (
          <div className="p-4 sm:p-6">
            <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
              <h2 className="text-2xl font-bold text-gray-900 mb-4">Audit Logs</h2>
              <div className="space-y-3">
                <div className="flex items-center justify-between py-3 border-b border-gray-100">
                  <div>
                    <p className="font-medium text-gray-900">Vault Unlocked</p>
                    <p className="text-sm text-gray-500">User logged in successfully</p>
                  </div>
                  <span className="text-sm text-gray-400">Just now</span>
                </div>
                <div className="flex items-center justify-between py-3 border-b border-gray-100">
                  <div>
                    <p className="font-medium text-gray-900">Entry Added</p>
                    <p className="text-sm text-gray-500">New entry for github.com</p>
                  </div>
                  <span className="text-sm text-gray-400">1 hour ago</span>
                </div>
              </div>
            </div>
          </div>
        );
      
      case 'settings':
        return (
          <div className="p-4 sm:p-6">
            <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
              <h2 className="text-2xl font-bold text-gray-900 mb-4">Settings</h2>
              <div className="space-y-6">
                <div>
                  <h3 className="text-lg font-semibold text-gray-900 mb-2">Security</h3>
                  <div className="space-y-3">
                    <label className="flex items-center">
                      <input type="checkbox" className="rounded border-gray-300 text-blue-600 focus:ring-blue-500" defaultChecked />
                      <span className="ml-2 text-sm text-gray-700">Auto-lock vault after 15 minutes</span>
                    </label>
                    <label className="flex items-center">
                      <input type="checkbox" className="rounded border-gray-300 text-blue-600 focus:ring-blue-500" />
                      <span className="ml-2 text-sm text-gray-700">Require master password for copying</span>
                    </label>
                  </div>
                </div>
                <div>
                  <h3 className="text-lg font-semibold text-gray-900 mb-2">Account</h3>
                  <button className="bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded-lg text-sm font-medium">
                    Change Master Password
                  </button>
                </div>
              </div>
            </div>
          </div>
        );
      
      default:
        return (
          <div className="p-4 sm:p-6">
            <EntryList 
              searchQuery={searchQuery}
              onEditEntry={handleEditEntry}
            />
          </div>
        );
    }
  };

  if (!authState.isAuthenticated) {
    return (
      <>
        <AuthFlow onAuthenticated={handleAuthenticated} />
        <Toaster position="top-right" />
      </>
    );
  }

  return (
    <>
      <DashboardLayout
        currentView={currentView}
        onViewChange={handleViewChange}
        onLogout={handleLogout}
        user={authState.user}
      >
        {renderContent()}
      </DashboardLayout>
      <Toaster position="top-right" />
    </>
  );
}
