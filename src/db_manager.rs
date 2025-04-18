use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseType {
    SQLite,
    PostgreSQL,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub enabled: bool,
    pub db_type: DatabaseType,
    pub connection_string: String,
    pub sqlite_path: Option<PathBuf>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            db_type: DatabaseType::SQLite,
            connection_string: String::new(),
            sqlite_path: None,
        }
    }
}

pub struct DbManager {
    config: DatabaseConfig,
    connection: Option<rusqlite::Connection>,
}

impl DbManager {
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            config,
            connection: None,
        }
    }

    pub fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.config.db_type {
            DatabaseType::SQLite => {
                if let Some(path) = &self.config.sqlite_path {
                    self.connection = Some(rusqlite::Connection::open(path)?);
                }
            }
            DatabaseType::PostgreSQL => {
                // Future
            }
        }
        Ok(())
    }

    pub fn import_csv(&self, table_name: &str, csv_data: &[Vec<String>]) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(conn) = &self.connection {
            // Todo(Billy)
            // > Create table dynamically based on CSV headers
            // > Insert data
        }
        Ok(())
    }
}