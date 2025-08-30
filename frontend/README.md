# PassMan Frontend

A modern, minimalistic password manager frontend built with Next.js 15, TypeScript, and Tailwind CSS.

## Features

### 🔐 Authentication System
- **Registration Flow**: First-time users are guided through account creation
- **Secure Login**: Authentication with master password
- **Session Management**: Secure token-based authentication
- **Auto-redirect**: Seamless navigation between authenticated and unauthenticated states

### 🔑 Password Management
- **Vault Management**: Secure storage and retrieval of password entries
- **Entry Categories**: Organize passwords by Personal, Work, Financial, Social, Gaming, Shopping, and Other
- **Custom Fields**: Add additional metadata to entries
- **Favorites**: Mark frequently used entries
- **Search & Filter**: Quickly find specific entries

### 🎲 Advanced Password Generator
- **Customizable Length**: Generate passwords from 4 to 128 characters
- **Character Types**: Include/exclude uppercase, lowercase, numbers, and symbols
- **Similar Character Exclusion**: Option to exclude visually similar characters (il1Lo0O)
- **Bulk Generation**: Generate multiple passwords at once (1, 3, 5, or 10)
- **Strength Analysis**: Real-time password strength assessment
- **One-Click Copy**: Copy passwords with visual feedback

### 🎨 Minimalistic UI Design
- **Clean Interface**: Reduced visual clutter with focus on functionality
- **Responsive Layout**: Works seamlessly on desktop and mobile devices
- **Intuitive Navigation**: Simple sidebar navigation with clear icons
- **Accessibility**: Proper ARIA labels and keyboard navigation support
- **Visual Feedback**: Toast notifications and loading states

### 🛡️ Security Features
- **Zero-Knowledge Architecture**: Passwords are encrypted client-side
- **Secure Generation**: Cryptographically secure password generation
- **Strength Validation**: Real-time password strength assessment
- **Secure Copy**: Safe clipboard operations with automatic clearing

## Technology Stack

- **Framework**: Next.js 15 with App Router
- **Language**: TypeScript for type safety
- **Styling**: Tailwind CSS for responsive design
- **Icons**: Lucide React for consistent iconography
- **Notifications**: React Hot Toast for user feedback
- **State Management**: React hooks and context
- **Security**: Client-side encryption and validation

## Getting Started

### Prerequisites
- Node.js 18+ 
- npm or yarn package manager

### Installation

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd PassMan/frontend
   ```

2. **Install dependencies**
   ```bash
   npm install
   ```

3. **Start development server**
   ```bash
   npm run dev
   ```

4. **Open in browser**
   Navigate to `http://localhost:3000`

### First-Time Setup

1. **Registration**: When you first visit the application, you'll be prompted to create an account
2. **Master Password**: Choose a strong master password - this encrypts all your data
3. **Vault Creation**: Your secure vault will be initialized automatically
4. **Start Adding Entries**: Begin storing your passwords securely

## Usage Guide

### Managing Passwords

1. **Adding New Entries**
   - Click the "+" button in the sidebar
   - Fill in service details (name, username, password, URL, notes)
   - Select appropriate category and add tags
   - Save the entry

2. **Editing Entries**
   - Click on any entry in the list
   - Modify the details as needed
   - Save changes

3. **Organizing Entries**
   - Use categories to group related passwords
   - Add tags for additional organization
   - Mark important entries as favorites

### Generating Secure Passwords

1. **Access Generator**
   - Click the "Generate" icon in the sidebar
   - Or use the generator when creating new entries

2. **Customize Settings**
   - Adjust password length (4-128 characters)
   - Select character types to include
   - Choose to exclude similar-looking characters
   - Set number of passwords to generate

3. **Generate and Use**
   - Click "Generate Password(s)"
   - Review strength indicators
   - Copy desired passwords with one click

### Security Best Practices

- **Use Strong Master Password**: Your master password protects everything
- **Enable Unique Passwords**: Generate unique passwords for each service
- **Regular Updates**: Periodically update important passwords
- **Backup Strategy**: Export vault data for backup purposes
- **Device Security**: Keep your device secure and up-to-date

## Development

### Project Structure
```
frontend/
├── src/
│   ├── app/                 # Next.js app router pages
│   ├── components/          # React components
│   │   ├── AuthFlow.tsx     # Authentication components
│   │   ├── DashboardLayout.tsx
│   │   ├── EntryForm.tsx
│   │   ├── EntryList.tsx
│   │   ├── PasswordGenerator.tsx
│   │   └── UnlockVault.tsx
│   ├── lib/                 # Utility functions
│   │   ├── utils.ts
│   │   └── vault-service.ts # Mock backend service
│   └── types/               # TypeScript definitions
├── public/                  # Static assets
└── package.json
```

### Available Scripts

- `npm run dev` - Start development server
- `npm run build` - Build for production
- `npm run start` - Start production server
- `npm run lint` - Run ESLint
- `npm run type-check` - Run TypeScript compiler check

### Backend Integration

This frontend is designed to work with the Rust backend server. The `vault-service.ts` currently contains mock implementations for development. To connect to the actual Rust backend:

1. Update API endpoints in `vault-service.ts`
2. Configure CORS in the Rust server
3. Update authentication flow to use JWT tokens
4. Implement proper error handling for network requests

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Security Notice

This is a demo application. For production use:
- Implement proper backend authentication
- Use secure HTTPS connections
- Follow security best practices for password storage
- Conduct security audits regularly
- Implement proper session management
