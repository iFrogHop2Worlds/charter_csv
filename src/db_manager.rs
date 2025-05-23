use rayon::prelude::*;
use std::error::Error;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use itertools::Itertools;
use rusqlite::Connection;
use crate::charter_utilities::{get_default_db_path, CsvGrid};

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseType {
    SQLite,
    PostgreSQL,
    MongoDB,
    CsvQB,
}

impl DatabaseType {
    pub fn is(&self, other: DatabaseType) -> bool {
        self == &other
    }
}

impl Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            DatabaseType::SQLite => "SQLite".to_string(),
            DatabaseType::PostgreSQL => "PostgreSQL".to_string(),
            DatabaseType::MongoDB => "MongoDB".to_string(),
            DatabaseType::CsvQB => "CsvQB".to_string(),
        };
        write!(f, "{}", str)
    }
}

impl FromStr for DatabaseType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SQLite" => Ok(DatabaseType::SQLite),
            "PostgreSQL" => Ok(DatabaseType::PostgreSQL),
            "MongoDB" => Ok(DatabaseType::MongoDB),
            "CsvQB" => Ok(DatabaseType::CsvQB),
            _ => Err(format!("Invalid database type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseSource {
    Default,
    Custom(PathBuf)
}
impl DatabaseSource {
    pub fn get_path(&self) -> PathBuf {
        match self {
            DatabaseSource::Default => get_default_db_path(),
            DatabaseSource::Custom(path) => path.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub enabled: bool,
    pub db_type: DatabaseType,
    pub connection_string: String,
    pub database_path: DatabaseSource,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            db_type: DatabaseType::SQLite,
            connection_string: String::new(),
            database_path: DatabaseSource::Default,
        }
    }
}

pub struct DbManager {
    config: DatabaseConfig,
    pub(crate) connection: Option<Connection>,
}

impl DbManager {
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            config,
            connection: None,
        }
    }

    pub fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        match self.config.db_type {
            DatabaseType::SQLite => {
                let path = self.config.database_path.get_path();
                self.connection = Some(Connection::open(path)?);
            }
            DatabaseType::PostgreSQL => {
                // Future
            }
            DatabaseType::MongoDB => {
                // Future
            }
            _ => {}
        }
        Ok(())
    }



    pub fn import_all_csvs(conn: &mut Connection, csv_files: &Vec<(String, CsvGrid)>) -> Result<(), Box<dyn Error>> {
        for (file_path, csv_grid) in csv_files {
            let table_name = file_path
                .split(['/', '\\'])
                .last()
                .and_then(|s| s.split('.').next())
                .ok_or("Invalid file path")?;

            DbManager::import_csv(conn, table_name, csv_grid)?;
        }
        Ok(())
    }
    pub fn import_csv(conn: &mut Connection, table_name: &str, csv_data: &CsvGrid) -> Result<(), Box<dyn Error>> {

        if csv_data.is_empty() {
            return Err("Empty CSV data".into());
        }

        let table_name = table_name.split(['/', '\\', '-', ' '])
            .map(|name| {
                name.chars()
                    .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
                    .collect::<String>()
            }).join("_");

        let check_table_sql = "SELECT name FROM sqlite_master WHERE type='table' AND name=?";

        let exists: bool = conn.query_row(check_table_sql, [&table_name], |_| Ok(true))
            .unwrap_or(false);

        if !exists {
            let headers = &csv_data[0].iter()
                .map(|h| {
                    let sanitized = h.chars()
                        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
                        .collect::<String>();

                    let lower_sanitized = sanitized.to_lowercase();

                    let column_name = if RESERVED_WORDS.contains(&lower_sanitized.as_str()) {
                        format!("{}_{}", table_name, sanitized)
                    } else {
                        sanitized
                    };

                    format!("{}", column_name)
                })
                .collect::<Vec<_>>()
                .join(", ");

            const RESERVED_WORDS: &[&str] = &[
                "index", "Index", "group", "order", "table", "select", "where", "from", "having", "update",
                "delete", "references", "ABORT", "ACTION", "ADD", "AFTER", "ALL", "ALTER", "ALWAYS", "ANALYZE",
                "AND", "AS", "ASC", "ATTACH", "AUTOINCREMENT", "BEFORE", "BEGIN", "BETWEEN", "BY", "CASCADE",
                "CASE", "CAST", "CHECK", "COLLATE", "COLUMN", "COMMIT", "CONFLICT", "CONSTRAINT", "CREATE",
                "CROSS", "CURRENT", "CURRENT_DATE", "CURRENT_TIME", "CURRENT_TIMESTAMP", "DATABASE", "DEFAULT",
                "DEFERRABLE", "DEFERRED", "DESC", "DETACH", "DISTINCT", "DO", "DROP", "EACH", "ELSE", "END",
                "ESCAPE", "EXCEPT", "EXCLUSIVE", "EXISTS", "EXPLAIN", "FAIL", "FILTER", "FIRST", "FOLLOWING",
                "FOR", "FOREIGN", "FULL", "GENERATED", "GLOB", "GROUPS", "IF", "IGNORE", "IMMEDIATE", "IN",
                "INDEXED", "INITIALLY", "INNER", "INSERT", "INSTEAD", "INTERSECT", "INTO", "IS", "ISNULL",
                "JOIN", "KEY", "LAST", "LEFT", "LIKE", "LIMIT", "MATCH", "NATURAL", "NO", "NOT", "NOTHING",
                "NOTNULL", "NULL", "NULLS", "OF", "OFFSET", "ON", "OR", "OTHERS", "OUTER", "OVER", "PARTITION",
                "PLAN", "PRAGMA", "PRECEDING", "PRIMARY", "QUERY", "RAISE", "RANGE", "RECURSIVE", "REGEXP",
                "REINDEX", "RELEASE", "RENAME", "REPLACE", "RESTRICT", "RETURNING", "RIGHT", "ROLLBACK", "ROW",
                "ROWS", "SAVEPOINT", "SET", "STORED", "TEMP", "TEMPORARY", "THEN", "TIES", "TO", "TRANSACTION",
                "TRIGGER", "UNBOUNDED", "UNION", "UNIQUE", "USING", "VACUUM", "VALUES", "VIEW", "VIRTUAL",
                "WHEN", "WINDOW", "WITH", "WITHOUT"
            ];

            let create_table_sql = format!(
                "CREATE TABLE {:?} ({})",
                table_name,
                headers
            );

            let _ = conn.execute(&create_table_sql, [])?;

            let column_count = headers.split(',').count();
            let insert_sql = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table_name,
                headers,
                (0..column_count).map(|_| "?").collect::<Vec<_>>().join(", ")
            );

            let tx = conn.transaction()?;
            {
                let mut stmt = tx.prepare(&insert_sql)?;
                for row in csv_data.iter().skip(1) {
                    if row.len() != column_count {
                        return Err(Box::new(rusqlite::Error::InvalidParameterCount(row.len(), column_count)));
                    }

                    let params: Vec<&dyn rusqlite::ToSql> = row
                        .iter()
                        .map(|s| s as &dyn rusqlite::ToSql)
                        .collect();

                    stmt.execute(params.as_slice())?;
                }
            }

            tx.commit()?;
        }

        Ok(())
    }

    pub fn load_file_from_db(conn: &mut Connection, file_name: &str) -> Result<(String, CsvGrid), Box<dyn Error>> {
        let content_query = format!("SELECT * FROM \"{}\"", file_name);
        let mut content_stmt = conn.prepare(&content_query)?;
        let mut grid: CsvGrid = Vec::new();

        let column_names: Vec<String> = content_stmt
            .column_names()
            .into_iter()
            .map(String::from)
            .collect();

        grid.push(column_names);

        let column_count = content_stmt.column_count();
        let rows = content_stmt.query_map([], |row| {
            let mut row_data = Vec::new();
            for i in 0..column_count {
                let value: String = row.get(i)?;
                row_data.push(value);
            }
            Ok(row_data)
        })?;

        for row_result in rows {
            grid.push(row_result?);
        }

        let path = format!("{}.csv", file_name);

        Ok((path, grid))
    }

}