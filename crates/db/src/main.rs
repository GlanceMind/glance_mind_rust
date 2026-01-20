//! GlanceMind Database Management CLI
//!
//! This is the main entry point for database management operations.
//! For specific operations, use the dedicated binaries:
//!   - db-migrate: Run database migrations
//!   - db-schema: Manage database schema

fn main() {
    println!("GlanceMind Database Management");
    println!("==============================\n");
    println!("Available commands:");
    println!("  cargo run --bin db-migrate          Run pending migrations");
    println!("  cargo run --bin db-migrate -- -s    Show migration status");
    println!("  cargo run --bin db-schema           Print database schema");
    println!("  cargo run --bin db-schema -- -v     Validate schema");
    println!();
    println!("Or use diesel CLI directly:");
    println!("  diesel migration run                Run migrations");
    println!("  diesel migration revert             Revert last migration");
    println!("  diesel migration generate <name>    Create new migration");
    println!("  diesel print-schema                 Print database schema");
}
