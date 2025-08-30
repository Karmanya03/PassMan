# PassMan - Secure Password Manager

A full-stack password manager application built with Rust backend and Next.js frontend, featuring military-grade encryption and modern web UI.

## 🚀 Features

### Backend (Rust)
- **Military-Grade Security**: XChaCha20Poly1305 encryption with Argon2id key derivation
- **Comprehensive CLI**: Full-featured command-line interface for password management
- **Security Auditing**: Detailed audit logs and security monitoring
- **Password Analysis**: Strength checking, breach detection, and security scoring
- **Export/Import**: JSON and CSV support for data portability
- **Performance**: High-performance cryptographic operations with Rust

### Frontend (Next.js)
- **Modern Web UI**: Clean, responsive interface built with Next.js 15 and Tailwind CSS
- **Registration Flow**: Guided account creation for first-time users
- **Minimalistic Design**: Clean, distraction-free interface focused on usability
- **Secure Vault**: Web-based password vault with unlock mechanism
- **Entry Management**: Add, edit, delete, and organize password entries
- **Advanced Password Generator**: Configurable generation with multiple character types
- **Search & Filter**: Quick search and category-based filtering
- **Visual Security**: Real-time password strength analysis and indicators
- **One-Click Operations**: Copy passwords with visual feedback
- **Accessibility**: Full keyboard navigation and screen reader support

## 🛡️ Security Features

- **Zero-Knowledge Architecture**: Master password never leaves your device
- **Strong Encryption**: XChaCha20Poly1305 authenticated encryption
- **Key Derivation**: Argon2id for secure password-based key derivation
- **Memory Safety**: Rust's memory safety prevents common vulnerabilities
- **Automatic Locking**: Time-based vault locking for session security
- **Audit Logging**: Comprehensive logging of all vault operations

## 🏗️ Architecture

```
PassMan/
├── src/                    # Rust backend
│   ├── main.rs            # CLI application entry point
│   ├── vault.rs           # Vault management and storage
│   ├── crypto.rs          # Cryptographic operations
│   ├── entry.rs           # Password entry data structures
│   └── security.rs        # Security features and auditing
├── frontend/              # Next.js frontend
│   ├── src/
│   │   ├── app/          # Next.js app router
│   │   ├── components/   # React components
│   │   ├── lib/          # Utilities and services
│   │   └── types/        # TypeScript type definitions
│   └── public/           # Static assets
└── target/               # Rust build artifacts
```

## 🚀 Getting Started

### Prerequisites

- **Rust** 1.70+ with Cargo
- **Node.js** 18+ with npm
- **Git** for version control

### Backend Setup

1. **Clone the repository**:
   ```bash
   git clone <repository-url>
   cd PassMan
   ```

2. **Build the Rust application**:
   ```bash
   cargo build --release
   ```

3. **Run the CLI application**:
   ```bash
   cargo run -- --help
   ```

### Frontend Setup

1. **Navigate to frontend directory**:
   ```bash
   cd frontend
   ```

2. **Install dependencies**:
   ```bash
   npm install
   ```

3. **Start development server**:
   ```bash
   npm run dev
   ```

4. **Open your browser**:
   Navigate to `http://localhost:3000`

## 💻 Usage

### Web Interface

1. **Access the application** at `http://localhost:3000`
2. **Unlock the vault** with your master password (demo: any 8+ character password)
3. **Manage entries** using the intuitive web interface:
   - View all password entries in the vault
   - Add new entries with the form
   - Edit existing entries
   - Generate secure passwords
   - Search and filter entries
   - Copy passwords to clipboard

### CLI Interface

```bash
# Add a new password entry
cargo run -- add gmail user@gmail.com --generate

# List all entries
cargo run -- list --detailed

# Search for entries
cargo run -- find github

# Generate secure passwords
cargo run -- generate --length 20 --symbols --count 5

# View vault statistics
cargo run -- stats

# Export vault data
cargo run -- export vault_backup.json

# Change master password
cargo run -- change-password
```

## 🔧 Configuration

### Environment Variables

```bash
# Skip master password prompt (use with caution)
export PASSMAN_MASTER_PASSWORD="your_master_password"

# Enable verbose logging
export RUST_LOG=debug
```

### Frontend Configuration

The frontend uses mock data for development. In production, you would:

1. Create a Rust web server (using frameworks like Axum or Warp)
2. Implement REST API endpoints
3. Update the `VaultService` to make HTTP requests
4. Add proper authentication and session management

## 📁 File Structure

### Rust Backend

- **`main.rs`**: CLI argument parsing and command routing
- **`vault.rs`**: Core vault operations (load, save, lock management)
- **`crypto.rs`**: Encryption, decryption, and key derivation
- **`entry.rs`**: Password entry data structures and operations
- **`security.rs`**: Audit logging and security features

### Next.js Frontend

- **`components/UnlockVault.tsx`**: Master password entry screen
- **`components/DashboardLayout.tsx`**: Main application layout
- **`components/EntryList.tsx`**: Password entry display and management
- **`components/EntryForm.tsx`**: Add/edit entry form
- **`components/PasswordGenerator.tsx`**: Password generation interface
- **`lib/vault-service.ts`**: API service layer (currently mock)
- **`lib/utils.ts`**: Utility functions for formatting and helpers

## 🔒 Security Considerations

### Development vs Production

This demo includes some development conveniences that should **NOT** be used in production:

1. **Mock Authentication**: The frontend uses simplified unlock logic
2. **Client-Side Storage**: Real implementation should store encrypted data server-side
3. **API Security**: Add proper authentication, rate limiting, and CSRF protection
4. **HTTPS**: Always use HTTPS in production
5. **Session Management**: Implement secure session handling

### Production Deployment

For production use:

1. **Create REST API**: Build a Rust web server with proper endpoints
2. **Database Integration**: Use a secure database for encrypted vault storage
3. **Authentication**: Implement proper user authentication and authorization
4. **Network Security**: Use HTTPS, implement CORS, and add security headers
5. **Audit Logging**: Enhance logging for security monitoring
6. **Backup Strategy**: Implement secure backup and recovery procedures

## 🧪 Development

### Running Tests

```bash
# Backend tests
cargo test

# Frontend tests
cd frontend && npm test
```

### Code Quality

```bash
# Rust formatting and linting
cargo fmt
cargo clippy

# Frontend linting
cd frontend && npm run lint
```

### Building for Production

```bash
# Backend release build
cargo build --release

# Frontend production build
cd frontend && npm run build
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔮 Future Enhancements

- [ ] REST API server implementation
- [ ] Database integration (PostgreSQL/SQLite)
- [ ] Multi-user support
- [ ] Mobile application (React Native)
- [ ] Browser extension
- [ ] Biometric authentication
- [ ] Hardware security key support
- [ ] Encrypted file attachments
- [ ] Password sharing between users
- [ ] Advanced reporting and analytics

## 📞 Support

For questions, issues, or contributions, please:

1. Check existing [Issues](../../issues)
2. Create a new issue with detailed information
3. Provide steps to reproduce any bugs
4. Include system information and error logs

---

**Note**: This is a demonstration project. For production use, additional security measures and proper infrastructure setup are required.
