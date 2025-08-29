mod vault;
mod crypto;
mod entry;
mod security;

use clap::{Parser, Subcommand, Args};
use std::process;
use std::io::{self, Write};

#[derive(Parser)]
#[command(name = "PassMan")]
#[command(version = "1.0")]
#[command(about = "🛡️ Military-Grade Password Manager")]
#[command(long_about = "A secure, high-performance password manager built with Rust, featuring XChaCha20Poly1305 encryption, Argon2id key derivation, and comprehensive security features.")]
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

    // Handle the command
    if let Err(e) = run_command(cli) {
        eprintln!("❌ Error: {}", e);
        process::exit(1);
    }
}

fn run_command(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let master_password = if cli.no_prompt {
        get_password_from_env()?
    } else {
        get_secure_master_password()?
    };

    if master_password.is_empty() {
        return Err("Master password cannot be empty".into());
    }

    let mut vault = vault::Vault::load(&master_password).unwrap_or_else(|err| {
        if cli.verbose {
            eprintln!("⚠️  Could not load existing vault ({}), creating new one", err);
        }
        vault::Vault::new(900) // 15 minute timeout
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

fn get_secure_master_password() -> Result<String, Box<dyn std::error::Error>> {
    print!("🔐 Enter master password: ");
    io::stdout().flush()?;
    
    let password = security::get_secure_password("");
    
    if password.len() < 8 {
        return Err("Master password must be at least 8 characters long".into());
    }
    
    Ok(password)
}

fn get_password_from_env() -> Result<String, Box<dyn std::error::Error>> {
    std::env::var("PASSMAN_MASTER_PASSWORD")
        .map_err(|_| "PASSMAN_MASTER_PASSWORD environment variable not set".into())
}

fn handle_add(vault: &mut vault::Vault, args: AddArgs) -> Result<(), Box<dyn std::error::Error>> {
    let password = if args.generate || args.password.is_none() {
        let generated = crypto::generate_password(16, true);
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

fn handle_list(vault: &vault::Vault, args: ListArgs) -> Result<(), Box<dyn std::error::Error>> {
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

fn handle_find(vault: &vault::Vault, args: FindArgs) -> Result<(), Box<dyn std::error::Error>> {
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

fn handle_delete(vault: &mut vault::Vault, args: DeleteArgs) -> Result<(), Box<dyn std::error::Error>> {
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

fn handle_status(vault: &vault::Vault) -> Result<(), Box<dyn std::error::Error>> {
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
    let benchmark_time = crypto::benchmark_key_derivation();
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

fn handle_logs(vault: &vault::Vault, args: LogsArgs) -> Result<(), Box<dyn std::error::Error>> {
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

fn handle_generate(args: GenerateArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎲 Generating {} password(s):", args.count);
    println!("{:-<60}", "");
    
    for i in 0..args.count {
        let password = crypto::generate_password(args.length, args.symbols);
        let strength = crypto::estimate_password_strength(&password);
        
        println!("{:2}. 🔑 {} (Strength: {} - {})", 
            i + 1, password, strength.score, strength.level);
        
        if args.clipboard && i == 0 {
            copy_to_clipboard(&password)?;
            println!("     📋 Copied to clipboard");
        }
    }
    
    Ok(())
}

fn handle_export(vault: &vault::Vault, args: ExportArgs) -> Result<(), Box<dyn std::error::Error>> {
    let export_data = vault.export_entries(&args.format)?;
    std::fs::write(&args.output, export_data)?;
    
    println!("📤 Exported vault to '{}' in {} format", args.output, args.format);
    println!("⚠️  Keep exported file secure - it contains sensitive data!");
    
    Ok(())
}

fn handle_import(_vault: &mut vault::Vault, _args: ImportArgs) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement import functionality
    println!("📥 Import functionality coming soon!");
    Ok(())
}

fn handle_change_password(_vault: &mut vault::Vault, _current_password: &str) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement password change
    println!("🔄 Change password functionality coming soon!");
    Ok(())
}

fn handle_benchmark() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Running crypto benchmarks...");
    
    let iterations = 3;
    let mut total_time = std::time::Duration::new(0, 0);
    
    for i in 0..iterations {
        print!("Run {}/{}: ", i + 1, iterations);
        io::stdout().flush()?;
        
        let time = crypto::benchmark_key_derivation();
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

fn handle_stats(vault: &vault::Vault) -> Result<(), Box<dyn std::error::Error>> {
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
            let strength = crypto::estimate_password_strength(&entry.password);
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

fn handle_check_strength(password: &str) -> Result<(), Box<dyn std::error::Error>> {
    let strength = crypto::estimate_password_strength(password);
    
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

fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
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
    let strength = crypto::estimate_password_strength(password);
    match strength.score {
        0..=30 => "🔴 Weak".to_string(),
        31..=60 => "🟡 Fair".to_string(),
        61..=80 => "🟢 Good".to_string(),
        81..=100 => "🛡️ Strong".to_string(),
        _ => "❓ Unknown".to_string(),
    }
}
