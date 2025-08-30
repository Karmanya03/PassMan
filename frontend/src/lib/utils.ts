import { type ClassValue, clsx } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function formatDate(date: string): string {
  return new Date(date).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  });
}

export function getPasswordStrengthColor(score: number): string {
  switch (score) {
    case 0:
    case 1:
      return 'text-red-500';
    case 2:
      return 'text-orange-500';
    case 3:
      return 'text-yellow-500';
    case 4:
      return 'text-green-500';
    case 5:
      return 'text-emerald-500';
    default:
      return 'text-gray-500';
  }
}

export function getPasswordStrengthText(score: number): string {
  switch (score) {
    case 0:
      return 'Very Weak';
    case 1:
      return 'Weak';
    case 2:
      return 'Fair';
    case 3:
      return 'Good';
    case 4:
      return 'Strong';
    case 5:
      return 'Very Strong';
    default:
      return 'Unknown';
  }
}

export function getCategoryColor(category: string): string {
  const colors: Record<string, string> = {
    'PERSONAL': 'bg-blue-100 text-blue-800',
    'WORK': 'bg-green-100 text-green-800',
    'FINANCIAL': 'bg-yellow-100 text-yellow-800',
    'SOCIAL': 'bg-purple-100 text-purple-800',
    'GAMING': 'bg-pink-100 text-pink-800',
    'SHOPPING': 'bg-orange-100 text-orange-800',
    'OTHER': 'bg-gray-100 text-gray-800'
  };
  return colors[category] || colors['OTHER'];
}

export function copyToClipboard(text: string): Promise<void> {
  return navigator.clipboard.writeText(text);
}

export function maskPassword(password: string, show: boolean = false): string {
  if (show) return password;
  return '•'.repeat(Math.min(password.length, 12));
}
