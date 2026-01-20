//! Database migration CLI tool
//!
//! Usage:
//!   db-migrate              # Run pending migrations
//!   db-migrate --status     # Show migration status
//!   db-migrate --pending    # List pending migrations
//!   db-migrate --applied    # List applied migrations

use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel_migrations::MigrationHarness;
use std::env;

const MIGRATIONS: diesel_migrations::EmbeddedMigrations = diesel_migrations::embed_migrations!();

fn main() {
    dotenvy::dotenv().ok();

    let args: Vec<String> = env::args().collect();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("Connecting to database...");

    let mut conn = PgConnection::establish(&database_url)
        .unwrap_or_else(|e| panic!("Error connecting to {}: {}", database_url, e));

    println!("Connected successfully!\n");

    if args.len() > 1 {
        match args[1].as_str() {
            "--status" | "-s" => show_status(&mut conn),
            "--pending" | "-p" => show_pending(&mut conn),
            "--applied" | "-a" => show_applied(&mut conn),
            "--help" | "-h" => show_help(),
            _ => {
                eprintln!("Unknown option: {}", args[1]);
                show_help();
                std::process::exit(1);
            }
        }
    } else {
        run_migrations(&mut conn);
    }
}

fn run_migrations(conn: &mut PgConnection) {
    println!("Running pending migrations...\n");

    let pending = conn
        .pending_migrations(MIGRATIONS)
        .expect("Failed to get pending migrations");

    if pending.is_empty() {
        println!("No pending migrations. Database is up to date.");
        return;
    }

    println!("Pending migrations ({}):", pending.len());
    for migration in &pending {
        println!("  - {}", migration.name());
    }
    println!();

    match conn.run_pending_migrations(MIGRATIONS) {
        Ok(applied) => {
            println!("Successfully applied {} migration(s):", applied.len());
            for name in applied {
                println!("  - {}", name);
            }
        }
        Err(e) => {
            eprintln!("Migration failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn show_status(conn: &mut PgConnection) {
    let applied = conn
        .applied_migrations()
        .expect("Failed to get applied migrations");
    let pending = conn
        .pending_migrations(MIGRATIONS)
        .expect("Failed to get pending migrations");

    println!("=== Database Migration Status ===\n");
    println!("Applied migrations: {}", applied.len());
    println!("Pending migrations: {}", pending.len());
    println!();

    if pending.is_empty() {
        println!("Database schema is up to date!");
    } else {
        println!(
            "There are {} pending migration(s). Run 'db-migrate' to apply.",
            pending.len()
        );
    }
}

fn show_pending(conn: &mut PgConnection) {
    let pending = conn
        .pending_migrations(MIGRATIONS)
        .expect("Failed to get pending migrations");

    println!("=== Pending Migrations ===\n");

    if pending.is_empty() {
        println!("No pending migrations.");
    } else {
        for (i, migration) in pending.iter().enumerate() {
            println!("{}. {}", i + 1, migration.name());
        }
    }
}

fn show_applied(conn: &mut PgConnection) {
    let applied = conn
        .applied_migrations()
        .expect("Failed to get applied migrations");

    println!("=== Applied Migrations ===\n");

    if applied.is_empty() {
        println!("No migrations have been applied.");
    } else {
        for (i, migration) in applied.iter().enumerate() {
            println!("{}. {}", i + 1, migration);
        }
    }
}

fn show_help() {
    println!("GlanceMind Database Migration Tool\n");
    println!("Usage: db-migrate [OPTIONS]\n");
    println!("Options:");
    println!("  (no option)     Run all pending migrations");
    println!("  --status, -s    Show migration status");
    println!("  --pending, -p   List pending migrations");
    println!("  --applied, -a   List applied migrations");
    println!("  --help, -h      Show this help message");
}
