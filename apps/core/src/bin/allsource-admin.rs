/// AllSource Admin CLI Tool
///
/// Command-line interface for managing AllSource v1.0
/// Features: User management, Tenant management, Backups, Statistics

use allsource_core::{
    auth::{AuthManager, Role},
    backup::{BackupConfig, BackupManager},
    config::Config,
    tenant::TenantManager,
};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug)]
enum Command {
    // User commands
    UserCreate { username: String, email: String, password: String, role: Role },
    UserList,
    UserDelete { username: String },

    // Tenant commands
    TenantCreate { id: String, name: String, tier: String },
    TenantList,
    TenantStats { id: String },
    TenantDeactivate { id: String },

    // Backup commands
    BackupCreate,
    BackupList,
    BackupRestore { backup_id: String },

    // System commands
    Config { show: bool, generate: bool },
    Stats,
    Help,
}

fn print_help() {
    println!(r#"
AllSource Admin CLI v1.0

USAGE:
    allsource-admin <COMMAND> [OPTIONS]

USER COMMANDS:
    user create <username> <email> <password> [role]
        Create a new user (role: admin, developer, readonly, service)

    user list
        List all users

    user delete <username>
        Delete a user

TENANT COMMANDS:
    tenant create <id> <name> [tier]
        Create a new tenant (tier: free, professional, unlimited)

    tenant list
        List all tenants

    tenant stats <id>
        Show tenant statistics and quota usage

    tenant deactivate <id>
        Deactivate a tenant

BACKUP COMMANDS:
    backup create
        Create a full backup

    backup list
        List all backups

    backup restore <backup_id>
        Restore from a backup

SYSTEM COMMANDS:
    config show
        Display current configuration

    config generate
        Generate example configuration file

    stats
        Show system statistics

    help
        Show this help message

EXAMPLES:
    # Create an admin user
    allsource-admin user create admin admin@example.com secret123 admin

    # Create a professional tenant
    allsource-admin tenant create acme "Acme Corp" professional

    # Create a backup
    allsource-admin backup create

    # Show configuration
    allsource-admin config show

For more information, visit: https://docs.allsource.io
"#);
}

fn parse_args() -> Result<Command> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        return Ok(Command::Help);
    }

    match args[1].as_str() {
        "user" => {
            if args.len() < 3 {
                return Ok(Command::Help);
            }
            match args[2].as_str() {
                "create" => {
                    if args.len() < 6 {
                        anyhow::bail!("Usage: user create <username> <email> <password> [role]");
                    }
                    let role = if args.len() > 6 {
                        match args[6].as_str() {
                            "admin" => Role::Admin,
                            "developer" => Role::Developer,
                            "readonly" => Role::ReadOnly,
                            "service" => Role::ServiceAccount,
                            _ => Role::Developer,
                        }
                    } else {
                        Role::Developer
                    };
                    Ok(Command::UserCreate {
                        username: args[3].clone(),
                        email: args[4].clone(),
                        password: args[5].clone(),
                        role,
                    })
                }
                "list" => Ok(Command::UserList),
                "delete" => {
                    if args.len() < 4 {
                        anyhow::bail!("Usage: user delete <username>");
                    }
                    Ok(Command::UserDelete {
                        username: args[3].clone(),
                    })
                }
                _ => Ok(Command::Help),
            }
        }
        "tenant" => {
            if args.len() < 3 {
                return Ok(Command::Help);
            }
            match args[2].as_str() {
                "create" => {
                    if args.len() < 5 {
                        anyhow::bail!("Usage: tenant create <id> <name> [tier]");
                    }
                    let tier = if args.len() > 5 {
                        args[5].clone()
                    } else {
                        "professional".to_string()
                    };
                    Ok(Command::TenantCreate {
                        id: args[3].clone(),
                        name: args[4].clone(),
                        tier,
                    })
                }
                "list" => Ok(Command::TenantList),
                "stats" => {
                    if args.len() < 4 {
                        anyhow::bail!("Usage: tenant stats <id>");
                    }
                    Ok(Command::TenantStats {
                        id: args[3].clone(),
                    })
                }
                "deactivate" => {
                    if args.len() < 4 {
                        anyhow::bail!("Usage: tenant deactivate <id>");
                    }
                    Ok(Command::TenantDeactivate {
                        id: args[3].clone(),
                    })
                }
                _ => Ok(Command::Help),
            }
        }
        "backup" => {
            if args.len() < 3 {
                return Ok(Command::Help);
            }
            match args[2].as_str() {
                "create" => Ok(Command::BackupCreate),
                "list" => Ok(Command::BackupList),
                "restore" => {
                    if args.len() < 4 {
                        anyhow::bail!("Usage: backup restore <backup_id>");
                    }
                    Ok(Command::BackupRestore {
                        backup_id: args[3].clone(),
                    })
                }
                _ => Ok(Command::Help),
            }
        }
        "config" => {
            if args.len() < 3 {
                return Ok(Command::Help);
            }
            match args[2].as_str() {
                "show" => Ok(Command::Config { show: true, generate: false }),
                "generate" => Ok(Command::Config { show: false, generate: true }),
                _ => Ok(Command::Help),
            }
        }
        "stats" => Ok(Command::Stats),
        "help" | "--help" | "-h" => Ok(Command::Help),
        _ => Ok(Command::Help),
    }
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let command = parse_args()?;

    match command {
        Command::UserCreate { username, email, password, role } => {
            println!("Creating user: {} ({})", username, email);
            let auth_manager = Arc::new(AuthManager::default());
            let user = auth_manager.register_user(
                username.clone(),
                email,
                &password,
                role.clone(),
                "default".to_string(),
            )?;
            println!("✅ User created successfully!");
            println!("   ID: {}", user.id);
            println!("   Username: {}", user.username);
            println!("   Role: {:?}", role);
        }

        Command::UserList => {
            println!("Listing all users...");
            let auth_manager = Arc::new(AuthManager::default());
            let users = auth_manager.list_users();
            println!("\nTotal users: {}\n", users.len());
            for user in users {
                println!("  • {} ({}) - Role: {:?}, Tenant: {}",
                    user.username, user.email, user.role, user.tenant_id);
            }
        }

        Command::UserDelete { username } => {
            println!("Deleting user: {}", username);
            let auth_manager = Arc::new(AuthManager::default());
            // Find user by username first
            let users = auth_manager.list_users();
            if let Some(user) = users.iter().find(|u| u.username == username) {
                auth_manager.delete_user(&user.id)?;
                println!("✅ User deleted successfully!");
            } else {
                println!("❌ User not found: {}", username);
            }
        }

        Command::TenantCreate { id, name, tier } => {
            println!("Creating tenant: {} ({})", id, name);
            let tenant_manager = Arc::new(TenantManager::new());
            let quotas = match tier.as_str() {
                "free" => allsource_core::tenant::TenantQuotas::free_tier(),
                "professional" => allsource_core::tenant::TenantQuotas::professional(),
                "unlimited" => allsource_core::tenant::TenantQuotas::unlimited(),
                _ => allsource_core::tenant::TenantQuotas::professional(),
            };
            let tenant = tenant_manager.create_tenant(id.clone(), name, quotas)?;
            println!("✅ Tenant created successfully!");
            println!("   ID: {}", tenant.id);
            println!("   Tier: {}", tier);
        }

        Command::TenantList => {
            println!("Listing all tenants...");
            let tenant_manager = Arc::new(TenantManager::new());
            let tenants = tenant_manager.list_tenants();
            println!("\nTotal tenants: {}\n", tenants.len());
            for tenant in tenants {
                println!("  • {} - {} (Active: {})",
                    tenant.id, tenant.name, tenant.active);
            }
        }

        Command::TenantStats { id } => {
            println!("Fetching stats for tenant: {}", id);
            let tenant_manager = Arc::new(TenantManager::new());
            let stats = tenant_manager.get_stats(&id)?;
            println!("\n{}", serde_json::to_string_pretty(&stats)?);
        }

        Command::TenantDeactivate { id } => {
            println!("Deactivating tenant: {}", id);
            let tenant_manager = Arc::new(TenantManager::new());
            tenant_manager.deactivate_tenant(&id)?;
            println!("✅ Tenant deactivated successfully!");
        }

        Command::BackupCreate => {
            println!("Creating backup...");
            let config = BackupConfig::default();
            let manager = BackupManager::new(config)?;
            // Note: In real usage, you'd pass actual events from the store
            println!("⚠️  Backup creation requires event store access");
            println!("    Use the API endpoint: POST /api/v1/backups");
        }

        Command::BackupList => {
            println!("Listing backups...");
            let config = BackupConfig::default();
            let manager = BackupManager::new(config)?;
            let backups = manager.list_backups()?;
            println!("\nTotal backups: {}\n", backups.len());
            for backup in backups {
                println!("  • {} - {} events ({} bytes)",
                    backup.backup_id, backup.event_count, backup.size_bytes);
                println!("    Created: {}", backup.created_at);
            }
        }

        Command::BackupRestore { backup_id } => {
            println!("Restoring from backup: {}", backup_id);
            let config = BackupConfig::default();
            let manager = BackupManager::new(config)?;
            let events = manager.restore_from_backup(&backup_id)?;
            println!("✅ Restored {} events successfully!", events.len());
        }

        Command::Config { show, generate } => {
            if show {
                println!("Current configuration:");
                match Config::load(None) {
                    Ok(config) => {
                        println!("\n{}", toml::to_string_pretty(&config)?);
                    }
                    Err(e) => {
                        println!("❌ Failed to load config: {}", e);
                    }
                }
            } else if generate {
                println!("# AllSource v1.0 Configuration Example\n");
                println!("{}", Config::example());
                println!("\n# Save this to config.toml and customize as needed");
            }
        }

        Command::Stats => {
            println!("System Statistics");
            println!("=================\n");

            let auth_manager = Arc::new(AuthManager::default());
            let tenant_manager = Arc::new(TenantManager::new());

            let users = auth_manager.list_users();
            let tenants = tenant_manager.list_tenants();

            println!("Users:   {}", users.len());
            println!("Tenants: {}", tenants.len());
            println!("\nTenant Breakdown:");
            for tenant in tenants {
                println!("  • {}: {} (Active: {})", tenant.id, tenant.name, tenant.active);
            }
        }

        Command::Help => {
            print_help();
        }
    }

    Ok(())
}
