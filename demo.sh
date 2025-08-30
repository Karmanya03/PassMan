#!/bin/bash

# PassMan CLI Demo Script
# This script demonstrates the Rust CLI functionality

echo "🛡️ PassMan CLI Demo"
echo "===================="

echo ""
echo "📋 Available Commands:"
echo "----------------------"

# Show help
echo "1. Getting help:"
cargo run -- --help

echo ""
echo "2. Adding a new entry:"
echo "cargo run -- add \"Gmail\" \"user@gmail.com\" \"SecurePassword123!\" --verbose"

echo ""
echo "3. Listing entries:"
echo "cargo run -- list --detailed"

echo ""
echo "4. Searching entries:"
echo "cargo run -- find \"gmail\""

echo ""
echo "5. Generating passwords:"
echo "cargo run -- generate --length 16 --symbols --count 3"

echo ""
echo "6. Checking password strength:"
echo "cargo run -- check-strength \"mypassword123\""

echo ""
echo "7. Viewing vault statistics:"
echo "cargo run -- stats"

echo ""
echo "8. Viewing audit logs:"
echo "cargo run -- logs --count 5"

echo ""
echo "9. Benchmarking crypto performance:"
echo "cargo run -- benchmark"

echo ""
echo "💡 Tips:"
echo "--------"
echo "• Use --verbose flag for detailed logging"
echo "• Master password is required for most operations"
echo "• Vault automatically locks after 15 minutes of inactivity"
echo "• All data is encrypted with XChaCha20Poly1305"
echo "• Use strong master passwords (12+ characters recommended)"

echo ""
echo "🔗 Frontend Interface:"
echo "---------------------"
echo "• Web UI: http://localhost:3000"
echo "• Demo master password: any 8+ character string"
echo "• Features: vault management, password generation, search"
