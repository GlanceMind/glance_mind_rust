//! Schema export and validation tool
//!
//! Usage:
//!   db-schema              # Print current schema
//!   db-schema --validate   # Validate schema matches database
//!   db-schema --diff       # Show differences between code and database

use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::env;
use std::process::Command;

fn main() {
    dotenvy::dotenv().ok();

    let args: Vec<String> = env::args().collect();
    
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    if args.len() > 1 {
        match args[1].as_str() {
            "--validate" | "-v" => validate_schema(&database_url),
            "--diff" | "-d" => show_diff(&database_url),
            "--export" | "-e" => export_schema(&database_url),
            "--help" | "-h" => show_help(),
            _ => {
                eprintln!("Unknown option: {}", args[1]);
                show_help();
                std::process::exit(1);
            }
        }
    } else {
        print_schema(&database_url);
    }
}

fn print_schema(database_url: &str) {
    println!("Connecting to database...");
    
    let _conn = PgConnection::establish(database_url)
        .unwrap_or_else(|e| panic!("Error connecting to {}: {}", database_url, e));
    
    println!("Running diesel print-schema...\n");
    
    let output = Command::new("diesel")
        .args(["print-schema"])
        .output()
        .expect("Failed to execute diesel print-schema");
    
    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }
}

fn validate_schema(database_url: &str) {
    println!("Validating schema...\n");
    
    let _conn = PgConnection::establish(database_url)
        .unwrap_or_else(|e| panic!("Error connecting to {}: {}", database_url, e));
    
    // Get current schema from database
    let output = Command::new("diesel")
        .args(["print-schema"])
        .output()
        .expect("Failed to execute diesel print-schema");
    
    if !output.status.success() {
        eprintln!("Error getting schema from database");
        std::process::exit(1);
    }
    
    let db_schema = String::from_utf8_lossy(&output.stdout);
    
    // Read current schema file
    let file_schema = std::fs::read_to_string("src/schema.rs")
        .expect("Failed to read src/schema.rs");
    
    // Compare (simplified - just check if they match)
    if db_schema.trim() == file_schema.trim() {
        println!("Schema is valid! Code matches database.");
    } else {
        println!("Schema mismatch detected!");
        println!("\nRun 'db-schema --diff' to see differences.");
        println!("Run 'db-schema --export' to update src/schema.rs");
        std::process::exit(1);
    }
}

fn show_diff(database_url: &str) {
    println!("Comparing schema with database...\n");
    
    let _conn = PgConnection::establish(database_url)
        .unwrap_or_else(|e| panic!("Error connecting to {}: {}", database_url, e));
    
    // Get current schema from database
    let output = Command::new("diesel")
        .args(["print-schema"])
        .output()
        .expect("Failed to execute diesel print-schema");
    
    if !output.status.success() {
        eprintln!("Error getting schema from database");
        std::process::exit(1);
    }
    
    // Write to temp file
    let temp_path = "/tmp/db_schema_current.rs";
    std::fs::write(temp_path, &output.stdout).expect("Failed to write temp file");
    
    // Run diff
    let diff_output = Command::new("diff")
        .args(["-u", "src/schema.rs", temp_path])
        .output()
        .expect("Failed to run diff");
    
    if diff_output.status.success() {
        println!("No differences found. Schema is in sync.");
    } else {
        println!("Differences found:\n");
        println!("{}", String::from_utf8_lossy(&diff_output.stdout));
    }
    
    // Cleanup
    let _ = std::fs::remove_file(temp_path);
}

fn export_schema(database_url: &str) {
    println!("Exporting schema from database...\n");
    
    let _conn = PgConnection::establish(database_url)
        .unwrap_or_else(|e| panic!("Error connecting to {}: {}", database_url, e));
    
    let output = Command::new("diesel")
        .args(["print-schema"])
        .output()
        .expect("Failed to execute diesel print-schema");
    
    if output.status.success() {
        std::fs::write("src/schema.rs", &output.stdout)
            .expect("Failed to write src/schema.rs");
        println!("Schema exported to src/schema.rs");
    } else {
        eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }
}

fn show_help() {
    println!("GlanceMind Database Schema Tool\n");
    println!("Usage: db-schema [OPTIONS]\n");
    println!("Options:");
    println!("  (no option)       Print current schema from database");
    println!("  --validate, -v    Validate code schema matches database");
    println!("  --diff, -d        Show differences between code and database");
    println!("  --export, -e      Export database schema to src/schema.rs");
    println!("  --help, -h        Show this help message");
}
