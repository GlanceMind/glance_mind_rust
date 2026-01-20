use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use std::env;

pub type DBPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct Database {
    pub pool: DBPool,
}

impl Database {
    pub fn new() -> Self {
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create pool");
        Database { pool }
    }

    pub fn get_pool(&self) -> DBPool {
        self.pool.clone()
    }
}

impl Default for Database {
    fn default() -> Self {
        // Warning: This will panic if DATABASE_URL is not set or DB is unreachable
        Self::new()
    }
}
