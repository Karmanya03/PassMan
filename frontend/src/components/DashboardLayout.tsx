'use client';

import { useState } from 'react';
import { 
  Shield, 
  Search, 
  Plus, 
  Key,
  BarChart3, 
  FileText,
  Settings,
  LogOut,
  Menu,
  X,
  Bell,
  User,
  Lock,
  Sparkles
} from 'lucide-react';
import { cn } from '@/lib/utils';
import toast from 'react-hot-toast';

interface DashboardLayoutProps {
  children: React.ReactNode;
  currentView: string;
  onViewChange: (view: string) => void;
  onLogout: () => void;
  user: any;
}

const navigation = [
  { id: 'vault', name: 'Password Vault', icon: Shield, color: 'from-blue-500 to-blue-600', bgColor: 'bg-blue-50 hover:bg-blue-100', textColor: 'text-blue-700' },
  { id: 'generate', name: 'Generator', icon: Key, color: 'from-purple-500 to-purple-600', bgColor: 'bg-purple-50 hover:bg-purple-100', textColor: 'text-purple-700' },
  { id: 'analytics', name: 'Analytics', icon: BarChart3, color: 'from-green-500 to-green-600', bgColor: 'bg-green-50 hover:bg-green-100', textColor: 'text-green-700' },
  { id: 'audit', name: 'Security Audit', icon: FileText, color: 'from-orange-500 to-orange-600', bgColor: 'bg-orange-50 hover:bg-orange-100', textColor: 'text-orange-700' },
  { id: 'settings', name: 'Settings', icon: Settings, color: 'from-gray-500 to-gray-600', bgColor: 'bg-gray-50 hover:bg-gray-100', textColor: 'text-gray-700' },
];

export default function DashboardLayout({ 
  children, 
  currentView, 
  onViewChange, 
  onLogout,
  user
}: DashboardLayoutProps) {
  const [sidebarOpen, setSidebarOpen] = useState(false);

  const handleLogout = () => {
    toast.success('Logged out successfully');
    onLogout();
  };

  return (
    <div className="flex h-screen bg-gradient-to-br from-slate-50 via-blue-50 to-indigo-100">
      {/* Mobile sidebar backdrop */}
      {sidebarOpen && (
        <div 
          className="fixed inset-0 bg-black/30 backdrop-blur-sm z-40 lg:hidden animate-fade-in"
          onClick={() => setSidebarOpen(false)}
        />
      )}

      {/* Sidebar */}
      <div className={cn(
        "fixed inset-y-0 left-0 z-50 w-80 transform transition-all duration-500 ease-out lg:translate-x-0 lg:static lg:inset-0",
        sidebarOpen ? 'translate-x-0' : '-translate-x-full'
      )}>
        <div className="h-full glass-effect shadow-2xl border-r-0">
          {/* Sidebar header */}
          <div className="flex items-center justify-between h-20 px-8 border-b border-white/20">
            <div className="flex items-center space-x-4">
              <div className="relative">
                <div className="w-12 h-12 bg-gradient-to-br from-blue-600 via-purple-600 to-indigo-600 rounded-2xl flex items-center justify-center shadow-lg">
                  <Lock className="w-6 h-6 text-white" />
                </div>
                <div className="absolute -top-1 -right-1 w-4 h-4 bg-gradient-to-r from-green-400 to-emerald-500 rounded-full border-2 border-white"></div>
              </div>
              <div>
                <h1 className="text-2xl font-bold gradient-text">PassMan</h1>
                <p className="text-xs text-gray-500 font-medium">Professional Edition</p>
              </div>
            </div>
            <button
              onClick={() => setSidebarOpen(false)}
              className="lg:hidden p-2 rounded-xl hover:bg-white/20 transition-all duration-200"
              aria-label="Close sidebar"
            >
              <X className="w-5 h-5 text-gray-600" />
            </button>
          </div>

          {/* User info */}
          <div className="px-8 py-6 border-b border-white/20">
            <div className="flex items-center space-x-4">
              <div className="relative">
                <div className="w-14 h-14 bg-gradient-to-br from-indigo-500 via-purple-500 to-pink-500 rounded-2xl flex items-center justify-center shadow-lg">
                  <User className="w-7 h-7 text-white" />
                </div>
                <div className="absolute -bottom-1 -right-1 w-5 h-5 bg-green-500 rounded-full border-2 border-white flex items-center justify-center">
                  <div className="w-2 h-2 bg-white rounded-full"></div>
                </div>
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-base font-semibold text-gray-900 truncate">
                  {user?.username || 'Professional User'}
                </p>
                <div className="flex items-center space-x-2 mt-1">
                  <Sparkles className="w-3 h-3 text-yellow-500" />
                  <span className="text-xs text-gray-600 font-medium">Premium Account</span>
                </div>
                <div className="flex items-center space-x-2 mt-1">
                  <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                  <span className="text-xs text-green-600 font-medium">Vault Secured</span>
                </div>
              </div>
              <div className="relative">
                <Bell className="w-5 h-5 text-gray-400 hover:text-gray-600 transition-colors cursor-pointer" />
                <div className="absolute -top-1 -right-1 w-3 h-3 bg-gradient-to-r from-red-500 to-pink-500 rounded-full animate-pulse"></div>
              </div>
            </div>
          </div>

          {/* Navigation */}
          <nav className="flex-1 px-6 py-6 space-y-3">
            {navigation.map((item) => {
              const Icon = item.icon;
              const isActive = currentView === item.id;
              
              return (
                <button
                  key={item.id}
                  onClick={() => {
                    onViewChange(item.id);
                    setSidebarOpen(false);
                  }}
                  className={cn(
                    "w-full flex items-center px-5 py-4 text-sm font-medium rounded-2xl transition-all duration-300 group relative overflow-hidden",
                    isActive
                      ? 'bg-gradient-to-r from-blue-600 to-purple-600 text-white shadow-lg transform scale-105 shadow-blue-500/25'
                      : 'text-gray-700 hover:bg-white/60 hover:shadow-lg hover:transform hover:scale-105'
                  )}
                >
                  <div className={cn(
                    "w-10 h-10 rounded-xl flex items-center justify-center mr-4 transition-all duration-300",
                    isActive 
                      ? "bg-white/20 shadow-lg" 
                      : "bg-white/40 group-hover:bg-white/60 group-hover:shadow-md"
                  )}>
                    <Icon className={cn(
                      "w-5 h-5 transition-colors",
                      isActive ? "text-white" : "text-gray-600 group-hover:text-gray-800"
                    )} />
                  </div>
                  <span className={cn(
                    "font-semibold",
                    isActive ? "text-white" : "text-gray-700 group-hover:text-gray-900"
                  )}>
                    {item.name}
                  </span>
                  {isActive && (
                    <div className="ml-auto flex items-center space-x-2">
                      <div className="w-2 h-2 bg-white rounded-full animate-pulse"></div>
                      <div className="w-1 h-1 bg-white/60 rounded-full"></div>
                    </div>
                  )}
                  {!isActive && (
                    <div className="ml-auto opacity-0 group-hover:opacity-100 transition-opacity">
                      <div className="w-2 h-2 bg-gray-400 rounded-full"></div>
                    </div>
                  )}
                </button>
              );
            })}
          </nav>

          {/* Logout button */}
          <div className="p-6 border-t border-white/20">
            <button
              onClick={handleLogout}
              className="w-full flex items-center px-5 py-4 text-sm font-medium text-red-600 hover:bg-red-50 rounded-2xl transition-all duration-300 hover:shadow-lg group"
            >
              <div className="w-10 h-10 rounded-xl bg-red-100 group-hover:bg-red-200 flex items-center justify-center mr-4 transition-all duration-300">
                <LogOut className="w-5 h-5 text-red-600" />
              </div>
              <span className="font-semibold">Sign Out</span>
            </button>
          </div>
        </div>
      </div>

      {/* Main content */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Header */}
        <header className="h-20 glass-effect border-b border-white/20 shadow-sm">
          <div className="flex items-center justify-between h-full px-8">
            <div className="flex items-center space-x-6">
              <button
                onClick={() => setSidebarOpen(true)}
                className="lg:hidden p-3 rounded-xl hover:bg-white/20 transition-all duration-200"
                aria-label="Open sidebar"
              >
                <Menu className="w-6 h-6 text-gray-600" />
              </button>
              
              <div>
                <h2 className="text-2xl font-bold text-gray-900 capitalize">
                  {currentView === 'vault' ? 'Password Vault' :
                   currentView === 'generate' ? 'Password Generator' :
                   currentView === 'analytics' ? 'Security Analytics' :
                   currentView === 'audit' ? 'Security Audit' :
                   currentView === 'settings' ? 'Settings' : currentView}
                </h2>
                <p className="text-sm text-gray-500 mt-1">
                  {currentView === 'vault' ? 'Manage and organize your secure passwords' :
                   currentView === 'generate' ? 'Create strong, unique passwords' :
                   currentView === 'analytics' ? 'Monitor your password security' :
                   currentView === 'audit' ? 'Review security events and logs' :
                   currentView === 'settings' ? 'Configure your preferences' : 'Professional password management'}
                </p>
              </div>
              
              {currentView === 'vault' && (
                <div className="relative max-w-md w-full ml-8">
                  <Search className="absolute left-4 top-1/2 transform -translate-y-1/2 text-gray-400 w-5 h-5" />
                  <input
                    type="text"
                    placeholder="Search your passwords..."
                    className="w-full pl-12 pr-4 py-3 border-2 border-gray-200 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-all duration-200 bg-white/80 backdrop-blur-sm"
                  />
                </div>
              )}
            </div>
            
            <div className="flex items-center space-x-4">
              <div className="hidden sm:flex items-center space-x-3 bg-white/60 backdrop-blur-sm rounded-xl px-4 py-2 border border-white/40">
                <div className="w-3 h-3 bg-green-500 rounded-full animate-pulse"></div>
                <span className="text-sm font-medium text-gray-700">Secure Session</span>
              </div>
              
              {currentView === 'vault' && (
                <button
                  onClick={() => onViewChange('add-entry')}
                  className="btn-gradient text-white px-6 py-3 rounded-xl flex items-center text-sm font-semibold shadow-lg hover:shadow-xl"
                >
                  <Plus className="w-4 h-4 mr-2" />
                  Add Entry
                </button>
              )}
            </div>
          </div>
        </header>

        {/* Content */}
        <main className="flex-1 overflow-y-auto p-8">
          <div className="animate-slide-up">
            {children}
          </div>
        </main>
      </div>
    </div>
  );
}
