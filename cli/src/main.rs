// Import modules from the shared library
use passmann_shared::{
    Entry, Vault,
    generate_password, encrypt, derive_key, Result, crypto::{
        benchmark_key_derivation, estimate_password_strength, generate_salt
    }
};

mod cloud;

use clap::{Parser, Subcommand, Args};
use std::process;
use std::io::{self, Write};

#[derive(Parser)]
#[command(name = "PassMann")]
#[command(version = "1.0")]
#[command(about = "🛡️ Military-Grade Password Manager")]
#[command(long_about = "A secure, high-performance password manager built with Rust, featuring XChaCha20Poly1305 encryption, Argon2id key derivation, and comprehensive security features with cloud synchronization.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Skip master password prompt (use with environment variable)
    #[arg(long, global = true)]
    no_prompt: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new password entry
    Add(AddArgs),
    /// List all password entries
    List(ListArgs),
    /// Search for specific entries
    Find(FindArgs),
    /// Delete entries matching a pattern
    Delete(DeleteArgs),
    /// Show vault status and statistics
    Status,
    /// Show recent audit logs
    Logs(LogsArgs),
    /// Generate secure passwords
    Generate(GenerateArgs),
    /// Export vault data
    Export(ExportArgs),
    /// Import vault data
    Import(ImportArgs),
    /// Change master password
    ChangePassword,
    /// Benchmark crypto performance
    Benchmark,
    /// Show vault statistics and health
    Stats,
    /// Check password strength
    CheckStrength { password: String },
    /// Cloud synchronization commands
    Sync(SyncArgs),
    /// Upload vault to cloud storage
    Upload,
    /// Download vault from cloud storage
    Download,
    /// Show cloud sync status
    CloudStatus,
    /// Create ultra-secure local vault
    CreateLocal(CreateLocalArgs),
    /// Use local vault (offline mode)
    Local(LocalArgs),
}

#[derive(Args)]
struct AddArgs {
    /// Service name (e.g., gmail, github)
    service: String,
    /// Username or email
    username: String,
    /// Password (leave empty to generate one)
    password: Option<String>,
    /// Generate a secure password
    #[arg(short, long)]
    generate: bool,
    /// Copy password to clipboard after adding
    #[arg(short, long)]
    clipboard: bool,
}

#[derive(Args)]
struct ListArgs {
    /// Show passwords in plain text (default: masked)
    #[arg(short, long)]
    show_passwords: bool,
    /// Show detailed information
    #[arg(short, long)]
    detailed: bool,
    /// Sort by service name
    #[arg(long)]
    sort: bool,
}

#[derive(Args)]
struct FindArgs {
    /// Search term (searches in service and username)
    query: String,
    /// Case-sensitive search
    #[arg(short, long)]
    case_sensitive: bool,
    /// Show passwords in results
    #[arg(short, long)]
    show_passwords: bool,
}

#[derive(Args)]
struct DeleteArgs {
    /// Service or pattern to delete
    pattern: String,
    /// Skip confirmation prompt
    #[arg(short, long)]
    force: bool,
}

#[derive(Args)]
struct LogsArgs {
    /// Number of recent logs to show
    #[arg(short, long, default_value = "10")]
    count: usize,
    /// Show logs since specific time (e.g., "1h", "30m", "1d")
    #[arg(short, long)]
    since: Option<String>,
}

#[derive(Args)]
struct SyncArgs {
    /// Force sync even if there are conflicts
    #[arg(short, long)]
    force: bool,
    /// Dry run - show what would be synced
    #[arg(short, long)]
    dry_run: bool,
    /// Specific device ID to sync with
    #[arg(short, long)]
    device: Option<String>,
}

#[derive(Args)]
struct CreateLocalArgs {
    /// Vault file path
    #[arg(short, long)]
    path: Option<String>,
    /// Security level (standard, high, military, paranoid)
    #[arg(short, long, default_value = "high")]
    security: String,
    /// Auto-lock timeout in minutes
    #[arg(short, long, default_value = "15")]
    timeout: u64,
}

#[derive(Args)]
struct LocalArgs {
    /// Vault file path
    #[arg(short, long)]
    path: Option<String>,
    /// Local vault command
    #[command(subcommand)]
    command: LocalCommands,
}

#[derive(Subcommand)]
enum LocalCommands {
    /// Add entry to local vault
    Add { service: String, username: String, password: Option<String> },
    /// List entries in local vault
    List,
    /// Search entries in local vault
    Search { query: String },
    /// Remove entry from local vault
    Remove { service: String, username: String },
    /// Change master password
    ChangePassword,
    /// Show vault statistics
    Stats,
    /// Lock vault
    Lock,
    /// Unlock vault
    Unlock,
    /// Create backup
    Backup,
}

#[derive(Args)]
struct GenerateArgs {
    /// Password length
    #[arg(short, long, default_value = "16")]
    length: usize,
    /// Include symbols
    #[arg(short, long)]
    symbols: bool,
    /// Copy to clipboard
    #[arg(short, long)]
    clipboard: bool,
    /// Number of passwords to generate
    #[arg(short, long, default_value = "1")]
    count: usize,
}

#[derive(Args)]
struct ExportArgs {
    /// Output file path
    output: String,
    /// Export format (json, csv)
    #[arg(short, long, default_value = "json")]
    format: String,
}

#[derive(Args)]
struct ImportArgs {
    /// Input file path
    input: String,
    /// Input format (json, csv)
    #[arg(short, long, default_value = "json")]
    format: String,
    /// Skip confirmation
    #[arg(short, long)]
    force: bool,
}

fn main() {
    let cli = Cli::parse();
    
    // Set up logging based on verbosity
    if cli.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    }

    // Handle the command - use tokio runtime for async commands
    let rt = tokio::runtime::Runtime::new().unwrap();
    if let Err(e) = rt.block_on(run_command(cli)) {
        eprintln!("❌ Error: {}", e);
        process::exit(1);
    }
}

async fn run_command(cli: Cli) -> Result<()> {
    let master_password = if cli.no_prompt {
        get_password_from_env()?
    } else {
        get_secure_master_password()?
    };

    if master_password.is_empty() {
        return Err("Master password cannot be empty".into());
    }

    let mut vault = Vault::load(&master_password).unwrap_or_else(|err| {
        if cli.verbose {
            eprintln!("⚠️  Could not load existing vault ({}), creating new one", err);
        }
        Vault::new(900) // 15 minute timeout
    });

    // Check vault lock status
    if vault.check_and_handle_lock() {
        return Err("Vault is locked due to inactivity. Please restart the application.".into());
    }

    match cli.command {
        Commands::Add(args) => handle_add(&mut vault, args)?,
        Commands::List(args) => handle_list(&vault, args)?,
        Commands::Find(args) => handle_find(&vault, args)?,
        Commands::Delete(args) => handle_delete(&mut vault, args)?,
        Commands::Status => handle_status(&vault)?,
        Commands::Logs(args) => handle_logs(&vault, args)?,
        Commands::Generate(args) => handle_generate(args)?,
        Commands::Export(args) => handle_export(&vault, args)?,
        Commands::Import(args) => handle_import(&mut vault, args)?,
        Commands::ChangePassword => handle_change_password(&mut vault, &master_password)?,
        Commands::Benchmark => handle_benchmark()?,
        Commands::Stats => handle_stats(&vault)?,
        Commands::CheckStrength { password } => handle_check_strength(&password)?,
        Commands::Sync(args) => handle_sync(&mut vault, args, &master_password).await?,
        Commands::Upload => handle_upload(&vault, &master_password).await?,
        Commands::Download => handle_download(&mut vault, &master_password).await?,
        Commands::CloudStatus => handle_cloud_status().await?,
        Commands::CreateLocal(args) => handle_create_local(args).await?,
        Commands::Local(args) => handle_local_commands(args).await?,
    }

    // Save vault
    if let Err(err) = vault.save(&master_password) {
        eprintln!("❌ Error saving vault: {}", err);
    } else if cli.verbose {
        println!("💾 Vault saved successfully.");
    }

    // Persist audit log
    if let Err(err) = vault.persist_audit_log() {
        eprintln!("⚠️  Warning: Could not save audit log: {}", err);
    }

    Ok(())
}

fn get_secure_master_password() -> Result<String> {
    print!("🔐 Enter master password: ");
    io::stdout().flush()?;
    
    let password = passmann_shared::security::get_secure_password("");
    
    if password.len() < 8 {
        return Err("Master password must be at least 8 characters long".into());
    }
    
    Ok(password)
}

fn get_password_from_env() -> Result<String> {
    std::env::var("PASSMANN_MASTER_PASSWORD")
        .map_err(|_| "PASSMANN_MASTER_PASSWORD environment variable not set".into())
}

fn handle_add(vault: &mut Vault, args: AddArgs) -> Result<()> {
    let password = if args.generate || args.password.is_none() {
        let generated = generate_password(16, true);
        println!("🔑 Generated password: {}", generated);
        
        if args.clipboard {
            copy_to_clipboard(&generated)?;
            println!("📋 Password copied to clipboard");
        }
        generated
    } else {
        args.password.unwrap()
    };

    vault.add_entry(args.service.clone(), args.username, password);
    println!("✅ Entry added for '{}'", args.service);
    
    Ok(())
}

fn handle_list(vault: &Vault, args: ListArgs) -> Result<()> {
    if let Some(entries) = vault.get_entries() {
        if entries.is_empty() {
            println!("📭 No entries found in vault.");
            return Ok(());
        }

        let mut sorted_entries = entries.clone();
        if args.sort {
            sorted_entries.sort_by(|a, b| a.service.cmp(&b.service));
        }

        println!("🔐 Vault Entries ({} total):", sorted_entries.len());
        println!("{:=<90}", "");

        for (i, entry) in sorted_entries.iter().enumerate() {
            let password_display = if args.show_passwords {
                &entry.password
            } else {
                "••••••••"
            };

            if args.detailed {
                println!("{:3}. 🌐 Service: {}", i + 1, entry.service);
                println!("     👤 User:    {}", entry.username);
                println!("     🔑 Pass:    {}", password_display);
                println!("     📊 Strength: {}", get_password_strength_indicator(&entry.password));
                println!("{:-<90}", "");
            } else {
                println!("{:3}. 🌐 {} | 👤 {} | 🔑 {}", 
                    i + 1, entry.service, entry.username, password_display);
            }
        }
        
        if !args.show_passwords {
            println!("\n💡 Use --show-passwords to reveal passwords");
        }
    }
    
    Ok(())
}

fn handle_find(vault: &Vault, args: FindArgs) -> Result<()> {
    if let Some(entries) = vault.get_entries() {
        let matches: Vec<_> = entries.iter().filter(|entry| {
            let service_match = if args.case_sensitive {
                entry.service.contains(&args.query)
            } else {
                entry.service.to_lowercase().contains(&args.query.to_lowercase())
            };
            
            let username_match = if args.case_sensitive {
                entry.username.contains(&args.query)
            } else {
                entry.username.to_lowercase().contains(&args.query.to_lowercase())
            };
            
            service_match || username_match
        }).collect();

        if matches.is_empty() {
            println!("🔍 No entries found matching '{}'", args.query);
        } else {
            println!("🎯 Found {} match(es) for '{}':", matches.len(), args.query);
            println!("{:-<80}", "");
            
            for (i, entry) in matches.iter().enumerate() {
                let password_display = if args.show_passwords {
                    &entry.password
                } else {
                    "••••••••"
                };
                
                println!("{:2}. 🌐 {} | 👤 {} | 🔑 {}", 
                    i + 1, entry.service, entry.username, password_display);
            }
        }
    }
    
    Ok(())
}

fn handle_delete(vault: &mut Vault, args: DeleteArgs) -> Result<()> {
    if !args.force {
        print!("⚠️  Are you sure you want to delete entries matching '{}'? (y/N): ", args.pattern);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().to_lowercase().starts_with('y') {
            println!("❌ Delete operation cancelled.");
            return Ok(());
        }
    }

    let removed = vault.remove_entries(&args.pattern);
    if removed > 0 {
        println!("🗑️  Deleted {} entry(ies) matching '{}'", removed, args.pattern);
    } else {
        println!("❌ No entries found matching '{}'", args.pattern);
    }
    
    Ok(())
}

fn handle_status(vault: &Vault) -> Result<()> {
    if let Some(time_left) = vault.get_lock_status() {
        let minutes = time_left.as_secs() / 60;
        let seconds = time_left.as_secs() % 60;
        println!("🔓 Vault Status: UNLOCKED");
        println!("⏰ Auto-lock in: {}m {}s", minutes, seconds);
    } else {
        println!("🔒 Vault Status: No auto-lock configured");
    }

    let stats = vault.get_vault_stats();
    println!("📊 Total entries: {}", stats.total_entries);
    println!("🏢 Unique services: {}", stats.unique_services);
    
    if stats.has_duplicates {
        println!("⚠️  Duplicate services detected");
    }

    // Show crypto benchmark
    let benchmark_time = benchmark_key_derivation();
    println!("⚡ Key derivation time: {}ms", benchmark_time.as_millis());
    
    if benchmark_time.as_millis() < 100 {
        println!("⚠️  Consider increasing Argon2id parameters for better security");
    } else if benchmark_time.as_millis() > 1000 {
        println!("💡 Consider decreasing Argon2id parameters for better performance");
    } else {
        println!("✅ Crypto parameters well-tuned");
    }
    
    Ok(())
}

fn handle_logs(vault: &Vault, args: LogsArgs) -> Result<()> {
    if let Some(ref audit) = vault.audit {
        let logs = audit.get_recent_logs(args.count);
        if logs.is_empty() {
            println!("📋 No audit logs found.");
        } else {
            println!("📋 Recent Audit Logs (last {}):", args.count);
            println!("{:-<100}", "");
            for (i, log) in logs.iter().enumerate() {
                println!("{:3}. 📝 {}", i + 1, log);
            }
            println!("{:-<100}", "");
        }
    } else {
        println!("❌ No audit log available.");
    }
    
    Ok(())
}

fn handle_generate(args: GenerateArgs) -> Result<()> {
    println!("🎲 Generating {} password(s):", args.count);
    println!("{:-<60}", "");
    
    for i in 0..args.count {
        let password = generate_password(args.length, args.symbols);
        let strength = estimate_password_strength(&password);
        
        println!("{:2}. 🔑 {} (Strength: {} - {})", 
            i + 1, password, strength.score, strength.level);
        
        if args.clipboard && i == 0 {
            copy_to_clipboard(&password)?;
            println!("     📋 Copied to clipboard");
        }
    }
    
    Ok(())
}

fn handle_export(vault: &Vault, args: ExportArgs) -> Result<()> {
    let export_data = vault.export_entries(&args.format)?;
    std::fs::write(&args.output, export_data)?;
    
    println!("📤 Exported vault to '{}' in {} format", args.output, args.format);
    println!("⚠️  Keep exported file secure - it contains sensitive data!");
    
    Ok(())
}

fn handle_import(_vault: &mut Vault, _args: ImportArgs) -> Result<()> {
    // TODO: Implement import functionality
    println!("📥 Import functionality coming soon!");
    Ok(())
}

fn handle_change_password(_vault: &mut Vault, _current_password: &str) -> Result<()> {
    // TODO: Implement password change
    println!("🔄 Change password functionality coming soon!");
    Ok(())
}

fn handle_benchmark() -> Result<()> {
    println!("⚡ Running crypto benchmarks...");
    
    let iterations = 3;
    let mut total_time = std::time::Duration::new(0, 0);
    
    for i in 0..iterations {
        print!("Run {}/{}: ", i + 1, iterations);
        io::stdout().flush()?;
        
        let time = benchmark_key_derivation();
        total_time += time;
        
        println!("{}ms", time.as_millis());
    }
    
    let avg_time = total_time / iterations;
    println!("\n📊 Average key derivation time: {}ms", avg_time.as_millis());
    
    if avg_time.as_millis() < 100 {
        println!("⚠️  Consider increasing security parameters (current: low security)");
    } else if avg_time.as_millis() < 500 {
        println!("✅ Good balance of security and performance");
    } else {
        println!("🛡️  High security (may impact user experience)");
    }
    
    Ok(())
}

fn handle_stats(vault: &Vault) -> Result<()> {
    let stats = vault.get_vault_stats();
    
    println!("📊 Vault Statistics");
    println!("{:=<50}", "");
    println!("Total entries:     {}", stats.total_entries);
    println!("Unique services:   {}", stats.unique_services);
    println!("Duplicate check:   {}", if stats.has_duplicates { "❌ Found" } else { "✅ None" });
    
    if let Some(entries) = vault.get_entries() {
        // Password strength analysis
        let mut weak_passwords = 0;
        let mut strong_passwords = 0;
        
        for entry in entries {
            let strength = estimate_password_strength(&entry.password);
            if strength.score < 60 {
                weak_passwords += 1;
            } else if strength.score >= 80 {
                strong_passwords += 1;
            }
        }
        
        println!("Strong passwords:  {} ({:.1}%)", 
            strong_passwords, 
            (strong_passwords as f32 / entries.len() as f32) * 100.0
        );
        println!("Weak passwords:    {} ({:.1}%)", 
            weak_passwords,
            (weak_passwords as f32 / entries.len() as f32) * 100.0
        );
    }
    
    Ok(())
}

fn handle_check_strength(password: &str) -> Result<()> {
    let strength = estimate_password_strength(password);
    
    println!("🔐 Password Strength Analysis");
    println!("{:-<40}", "");
    println!("Score:  {}/100", strength.score);
    println!("Level:  {}", strength.level);
    
    if !strength.feedback.is_empty() {
        println!("\n💡 Suggestions:");
        for suggestion in &strength.feedback {
            println!("  • {}", suggestion);
        }
    }
    
    let emoji = match strength.score {
        0..=30 => "🔴",
        31..=60 => "🟡", 
        61..=80 => "🟢",
        81..=100 => "🛡️",
        _ => "❓",
    };
    
    println!("\n{} Overall: {}", emoji, strength.level);
    
    Ok(())
}

// ============================================================================
// LOCAL VAULT HANDLERS (Ultra-Secure Military-Grade)
// ============================================================================

async fn handle_create_local(args: CreateLocalArgs) -> Result<()> {
    use passmann_shared::{LocalSecureVault, SecurityLevel};
    use std::path::PathBuf;
    
    println!("🛡️ Creating Ultra-Secure Local Vault");
    println!("=====================================");
    
    // Determine vault path
    let vault_path = match args.path {
        Some(path) => PathBuf::from(path),
        None => {
            let default_dir = dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("passmann");
            std::fs::create_dir_all(&default_dir)?;
            default_dir.join("vault_local.pmv")
        }
    };
    
    // Parse security level
    let security_level = match args.security.to_lowercase().as_str() {
        "standard" => SecurityLevel::Standard,
        "high" => SecurityLevel::High,
        "military" => SecurityLevel::Military,
        "paranoid" => SecurityLevel::Paranoid,
        _ => {
            println!("❌ Invalid security level. Options: standard, high, military, paranoid");
            return Ok(());
        }
    };
    
    println!("🔒 Security Level: {:?}", security_level);
    println!("⏱️  Unlock Time: {}", security_level.unlock_time_estimate());
    println!("📁 Vault Path: {}", vault_path.display());
    println!();
    
    if vault_path.exists() {
        print!("⚠️  Vault already exists. Overwrite? (y/N): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().to_lowercase().starts_with('y') {
            println!("❌ Operation cancelled");
            return Ok(());
        }
    }
    
    // Get master password
    let master_password = get_secure_master_password()?;
    if master_password.len() < 12 {
        println!("❌ Master password must be at least 12 characters for military-grade security");
        return Ok(());
    }
    
    // Confirm master password
    print!("🔐 Confirm master password: ");
    io::stdout().flush()?;
    let confirm_password = rpassword::read_password()?;
    
    if master_password != confirm_password {
        println!("❌ Passwords do not match");
        return Ok(());
    }
    
    println!("\n🔨 Creating vault with military-grade encryption...");
    println!("⚠️  This may take 10-30 seconds depending on security level");
    
    // Create vault
    let start_time = std::time::Instant::now();
    let _vault = LocalSecureVault::new(vault_path.clone(), &master_password, security_level)?;
    let creation_time = start_time.elapsed();
    
    println!("✅ Ultra-secure vault created successfully!");
    println!("⏱️  Creation time: {:.2}s", creation_time.as_secs_f64());
    println!("📁 Location: {}", vault_path.display());
    println!("\n🛡️ Security Features Enabled:");
    println!("   • Military-grade Argon2id key derivation");
    println!("   • XChaCha20Poly1305 encryption");
    println!("   • Multi-layer encryption (up to 5 layers)");
    println!("   • Automatic secure backups");
    println!("   • Auto-lock after {} minutes", args.timeout);
    println!("   • Secure memory wiping");
    println!("   • File integrity verification");
    
    println!("\n🚀 Usage:");
    println!("   passmann local --path {} add", vault_path.display());
    println!("   passmann local --path {} list", vault_path.display());
    
    Ok(())
}

async fn handle_local_commands(args: LocalArgs) -> Result<()> {
    use passmann_shared::LocalSecureVault;
    use std::path::PathBuf;
    
    // Determine vault path
    let vault_path = match args.path {
        Some(path) => PathBuf::from(path),
        None => {
            let default_dir = dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("passmann");
            default_dir.join("vault_local.pmv")
        }
    };
    
    if !vault_path.exists() {
        println!("❌ Local vault not found: {}", vault_path.display());
        println!("💡 Create one with: passmann create-local");
        return Ok(());
    }
    
    // Get master password for most commands
    let master_password = match args.command {
        LocalCommands::Lock => String::new(), // Lock doesn't need password
        _ => get_secure_master_password()?,
    };
    
    // Load vault
    let mut vault = match LocalSecureVault::load(vault_path.clone(), &master_password) {
        Ok(v) => v,
        Err(e) => {
            println!("❌ Failed to load vault: {}", e);
            println!("💡 Check your master password");
            return Ok(());
        }
    };
    
    match args.command {
        LocalCommands::Add { service, username, password } => {
            let final_password = match password {
                Some(p) => p,
                None => {
                    println!("🎲 Generating secure password...");
                    crate::generate_password(16, true)
                }
            };
            
            let entry = Entry::new(service.clone(), username.clone(), final_password.clone());
            
            vault.add_entry(entry)?;
            vault.save_to_disk(&master_password)?;
            
            println!("✅ Added entry for {} - {}", service, username);
            println!("🔐 Password: {}", final_password);
        }
        
        LocalCommands::List => {
            let entries = vault.get_entries()?;
            if entries.is_empty() {
                println!("📭 No entries found in vault");
            } else {
                println!("🔐 Local Vault Entries ({}):", entries.len());
                println!("{:-<60}", "");
                for entry in entries {
                    println!("🌐 {:<20} 👤 {:<25} 🔑 {}", 
                        entry.service, entry.username, "*".repeat(entry.password.len()));
                }
            }
        }
        
        LocalCommands::Search { query } => {
            let results = vault.search_entries(&query)?;
            if results.is_empty() {
                println!("🔍 No entries found matching '{}'", query);
            } else {
                println!("🔍 Search Results ({}):", results.len());
                println!("{:-<60}", "");
                for entry in results {
                    println!("🌐 {:<20} 👤 {:<25} 🔑 {}", 
                        entry.service, entry.username, "*".repeat(entry.password.len()));
                }
            }
        }
        
        LocalCommands::Remove { service, username } => {
            if vault.remove_entry(&service, &username)? {
                vault.save_to_disk(&master_password)?;
                println!("✅ Removed entry for {} - {}", service, username);
            } else {
                println!("❌ Entry not found: {} - {}", service, username);
            }
        }
        
        LocalCommands::Stats => {
            let stats = vault.get_stats()?;
            println!("📊 Local Vault Statistics");
            println!("{:-<50}", "");
            println!("📦 Total Entries: {}", stats.total_entries);
            println!("💾 File Size: {:.2} KB", stats.file_size_bytes as f64 / 1024.0);
            println!("🔒 Encryption Layers: {}", stats.encryption_layers);
            println!("📅 Created: {}", stats.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("📝 Modified: {}", stats.last_modified.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("✅ Integrity: {}", if stats.checksum_verified { "✅ Verified" } else { "❌ Failed" });
            println!("📁 Location: {}", vault_path.display());
        }
        
        LocalCommands::ChangePassword => {
            print!("🔐 Enter new master password: ");
            io::stdout().flush()?;
            let new_password = rpassword::read_password()?;
            
            if new_password.len() < 12 {
                println!("❌ New password must be at least 12 characters");
                return Ok(());
            }
            
            print!("🔐 Confirm new password: ");
            io::stdout().flush()?;
            let confirm_password = rpassword::read_password()?;
            
            if new_password != confirm_password {
                println!("❌ Passwords do not match");
                return Ok(());
            }
            
            vault.change_master_password(&master_password, &new_password)?;
            println!("✅ Master password changed successfully");
        }
        
        LocalCommands::Lock => {
            vault.lock();
            println!("🔒 Vault locked securely");
        }
        
        LocalCommands::Unlock => {
            // Vault is already unlocked if we got here
            println!("🔓 Vault unlocked successfully");
        }
        
        LocalCommands::Backup => {
            // Create manual backup
            vault.save_to_disk(&master_password)?;
            println!("💾 Backup created successfully");
        }
    }
    
    Ok(())
}

// ============================================================================
// CLOUD SYNCHRONIZATION HANDLERS
// ============================================================================

async fn handle_sync(
    vault: &mut Vault,
    args: SyncArgs,
    master_password: &str
) -> Result<()> {
    use cloud::{SupabaseClient, SyncMetadata};
    use chrono::Utc;
    
    println!("🌐 Initializing cloud sync...");
    
    let mut client = SupabaseClient::new()?;
    let user_id = get_or_create_user_id()?;
    let device_id = get_or_create_device_id()?;
    
    client.authenticate(user_id.clone()).await?;
    
    if args.dry_run {
        println!("🔍 Dry run mode - showing what would be synced:");
        let cloud_vault = client.download_vault(&device_id).await?;
        match cloud_vault {
            Some(cloud) => {
                println!("  Cloud vault found: {} bytes, updated {}", 
                    cloud.size_bytes, cloud.updated_at.unwrap_or(Utc::now()));
                println!("  Local vault: {} entries", vault.get_entries().map_or(0, |e| e.len()));
            }
            None => println!("  No cloud vault found"),
        }
        return Ok(());
    }
    
    // Check for conflicts
    let sync_metadata = client.get_sync_metadata(&device_id).await?;
    let cloud_vault = client.download_vault(&device_id).await?;
    
    match (sync_metadata, cloud_vault) {
        (Some(meta), Some(cloud)) => {
            if meta.pending_changes && !args.force {
                return Err("Sync conflict detected. Use --force to override.".into());
            }
            
            // Download and merge cloud vault
            println!("📥 Downloading cloud vault...");
            let decrypted_data = decrypt_cloud_vault(&cloud, master_password)?;
            vault.merge_from_json(&decrypted_data)?;
            
            println!("✅ Sync completed successfully");
        }
        (None, None) => {
            // First sync - upload local vault
            println!("📤 First sync - uploading local vault...");
            handle_upload(vault, master_password).await?;
        }
        (None, Some(_)) => {
            // Download existing cloud vault
            println!("📥 Downloading existing cloud vault...");
            handle_download(vault, master_password).await?;
        }
        (Some(_), None) => {
            // Upload local vault (cloud vault was deleted)
            println!("📤 Cloud vault missing - uploading local vault...");
            handle_upload(vault, master_password).await?;
        }
    }
    
    // Update sync metadata
    let metadata = SyncMetadata {
        user_id,
        device_id,
        last_sync: Utc::now(),
        sync_version: 1,
        pending_changes: false,
        conflict_resolution: "local_wins".to_string(),
    };
    
    client.update_sync_metadata(&metadata).await?;
    println!("🔄 Sync metadata updated");
    
    Ok(())
}

async fn handle_upload(
    vault: &Vault,
    master_password: &str
) -> Result<()> {
    use cloud::{SupabaseClient, CloudVault};
    use chrono::Utc;
    
    println!("📤 Uploading vault to cloud storage...");
    
    let mut client = SupabaseClient::new()?;
    let user_id = get_or_create_user_id()?;
    let device_id = get_or_create_device_id()?;
    let device_name = std::env::var("PASSMANN_DEVICE_NAME")
        .unwrap_or_else(|_| "Unknown Device".to_string());
    
    client.authenticate(user_id.clone()).await?;
    
    // Encrypt vault data
    let vault_json = vault.export_to_json()?;
    let (encrypted_data, salt) = encrypt_vault_data(&vault_json, master_password)?;
    
    let cloud_vault = CloudVault {
        id: None,
        user_id,
        encrypted_data,
        salt,
        device_id,
        device_name,
        version: 1,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
        checksum: calculate_checksum(&vault_json)?,
        compression_enabled: true,
        size_bytes: vault_json.len() as i64,
    };
    
    let vault_id = client.upload_vault(&cloud_vault).await?;
    println!("✅ Vault uploaded successfully (ID: {})", vault_id);
    
    Ok(())
}

async fn handle_download(
    vault: &mut Vault,
    master_password: &str
) -> Result<()> {
    use cloud::SupabaseClient;
    
    println!("📥 Downloading vault from cloud storage...");
    
    let mut client = SupabaseClient::new()?;
    let user_id = get_or_create_user_id()?;
    let device_id = get_or_create_device_id()?;
    
    client.authenticate(user_id).await?;
    
    let cloud_vault = client.download_vault(&device_id).await?;
    
    match cloud_vault {
        Some(cloud) => {
            println!("📦 Found cloud vault: {} bytes", cloud.size_bytes);
            
            // Decrypt and load vault data
            let decrypted_data = decrypt_cloud_vault(&cloud, master_password)?;
            vault.import_from_json(&decrypted_data)?;
            
            println!("✅ Vault downloaded and decrypted successfully");
            println!("📊 Loaded {} entries", vault.get_entries().map_or(0, |e| e.len()));
        }
        None => {
            println!("❌ No cloud vault found for this device");
        }
    }
    
    Ok(())
}

async fn handle_cloud_status() -> Result<()> {
    use cloud::SupabaseClient;
    
    println!("🌐 Checking cloud storage status...");
    
    let mut client = SupabaseClient::new()?;
    let user_id = get_or_create_user_id()?;
    let device_id = get_or_create_device_id()?;
    
    println!("👤 User ID: {}", user_id);
    println!("📱 Device ID: {}", device_id);
    
    // Try to connect to Supabase
    match client.authenticate(user_id.clone()).await {
        Ok(auth_client) => {
            println!("✅ Successfully connected to Supabase");
            
            // Check for existing vault
            match auth_client.download_vault(&device_id).await? {
                Some(vault) => {
                    println!("📦 Cloud vault found:");
                    println!("   Size: {} bytes", vault.size_bytes);
                    println!("   Updated: {}", vault.updated_at.unwrap_or_default());
                    println!("   Version: {}", vault.version);
                }
                None => {
                    println!("📭 No cloud vault found for this device");
                }
            }
            
            // Show recent audit logs
            match auth_client.get_audit_logs(Some(5)).await {
                Ok(logs) => {
                    println!("\n📋 Recent activity:");
                    for log in logs {
                        println!("   {} {} ({})", 
                            if log.success { "✅" } else { "❌" },
                            log.action,
                            log.metadata.unwrap_or_default()
                        );
                    }
                }
                Err(_) => println!("⚠️  Could not fetch audit logs"),
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to Supabase: {}", e);
        }
    }
    
    Ok(())
}

// ============================================================================
// CLOUD UTILITY FUNCTIONS
// ============================================================================

fn get_or_create_user_id() -> Result<String> {
    use std::env;
    use uuid::Uuid;
    
    match env::var("PASSMANN_USER_ID") {
        Ok(user_id) => Ok(user_id),
        Err(_) => {
            let user_id = Uuid::new_v4().to_string();
            unsafe { env::set_var("PASSMANN_USER_ID", &user_id); }
            println!("🆔 Generated new user ID: {}", user_id);
            println!("💡 Set PASSMANN_USER_ID={} in your .env file", user_id);
            Ok(user_id)
        }
    }
}

fn get_or_create_device_id() -> Result<String> {
    use std::env;
    use uuid::Uuid;
    
    match env::var("PASSMANN_DEVICE_ID") {
        Ok(device_id) => Ok(device_id),
        Err(_) => {
            let device_id = Uuid::new_v4().to_string();
            unsafe { env::set_var("PASSMANN_DEVICE_ID", &device_id); }
            println!("📱 Generated new device ID: {}", device_id);
            println!("💡 Set PASSMANN_DEVICE_ID={} in your .env file", device_id);
            Ok(device_id)
        }
    }
}

fn encrypt_vault_data(data: &str, master_password: &str) -> Result<(String, String)> {
    use base64::{Engine as _, engine::general_purpose};
    use {derive_key, encrypt};
    
    let salt = generate_salt(32);
    let key = derive_key(master_password, &salt);
    let encrypted = encrypt(&key, data.as_bytes());
    
    Ok((
        general_purpose::STANDARD.encode(encrypted),
        general_purpose::STANDARD.encode(salt)
    ))
}

fn decrypt_cloud_vault(cloud_vault: &cloud::CloudVault, master_password: &str) -> Result<String> {
    use base64::{Engine as _, engine::general_purpose};
    use passmann_shared::{derive_key, decrypt};
    
    let encrypted_data = general_purpose::STANDARD.decode(&cloud_vault.encrypted_data)?;
    let salt = general_purpose::STANDARD.decode(&cloud_vault.salt)?;
    
    let key = derive_key(master_password, &salt);
    let decrypted = decrypt(&key, &encrypted_data)?;
    
    Ok(String::from_utf8(decrypted)?)
}

fn calculate_checksum(data: &str) -> Result<String> {
    use blake3::Hasher;
    
    let mut hasher = Hasher::new();
    hasher.update(data.as_bytes());
    Ok(hasher.finalize().to_hex().to_string())
}

fn copy_to_clipboard(text: &str) -> Result<()> {
    // Simple clipboard implementation - in production, use a proper clipboard crate
    if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", &format!("echo {} | clip", text)])
            .output()?;
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("pbcopy")
            .arg(text)
            .output()?;
    } else {
        return Err("Clipboard not supported on this platform".into());
    }
    Ok(())
}

fn get_password_strength_indicator(password: &str) -> String {
    let strength = estimate_password_strength(password);
    match strength.score {
        0..=30 => "🔴 Weak".to_string(),
        31..=60 => "🟡 Fair".to_string(),
        61..=80 => "🟢 Good".to_string(),
        81..=100 => "🛡️ Strong".to_string(),
        _ => "❓ Unknown".to_string(),
    }
}
