use rayon::prelude::*;
use std::error::Error;
use std::io::ErrorKind;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use itertools::Itertools;
use rusqlite::{params, Connection};
use serde_json::Value;
use crate::charter_utilities::{csv_parser, get_default_db_path, CsvGrid};
use crate::session::{Session, SessionSummary};

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

    pub fn save_session_to_database(mut conn: Connection, sessions: Vec<Session>) -> Result<(), Box<dyn Error>> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
            name TEXT PRIMARY KEY,
            files TEXT,
            pipelines TEXT,
            selected_files TEXT,
            query_mode TEXT
        )",
            [],
        )?;

        let transaction = conn.transaction()?;

        for session in sessions {
            let files_json = serde_json::to_string(&session.files)?;
            let pipelines_json = serde_json::to_string(&session.pipelines)?;
            let selected_files_json = serde_json::to_string(&session.selected_files)?;
            let query_mode_str = format!("{:?}", session.query_mode);

            transaction.execute(
                "INSERT OR REPLACE INTO sessions (name, files, pipelines, selected_files, query_mode)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                session.name,
                files_json,
                pipelines_json,
                selected_files_json,
                query_mode_str,
            ],
            )?;
        }

        transaction.commit()?;

        Ok(())
    }

    pub fn retrieve_session_list(conn: &Connection) -> Result<Vec<SessionSummary>, Box<dyn Error>> {
        let mut stmt = conn.prepare(
            "SELECT name, files, pipelines, query_mode FROM sessions"
        ).or_else(|_| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS sessions (
                    name TEXT PRIMARY KEY,
                    files TEXT,
                    pipelines TEXT,
                    selected_files TEXT,
                    query_mode TEXT
                )",
                [],
            )?;
            conn.prepare("SELECT name, files, pipelines, query_mode FROM sessions")
        })?;

        let session_iter = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let files_json: String = row.get(1)?;
            let pipelines_json: String = row.get(2)?;
            let query_mode: String = row.get(3)?;

            let files: Value = serde_json::from_str(&files_json)
                .unwrap_or(Value::Array(vec![]));
            let pipelines: Value = serde_json::from_str(&pipelines_json)
                .unwrap_or(Value::Array(vec![]));

            let file_count = if let Value::Array(files_arr) = files {
                files_arr.len()
            } else {
                0
            };

            let pipeline_count = if let Value::Array(pipelines_arr) = pipelines {
                pipelines_arr.len()
            } else {
                0
            };

            Ok(SessionSummary {
                name,
                file_count,
                pipeline_count,
                query_mode,
            })
        })?;

        let mut sessions = Vec::new();
        for session in session_iter {
            sessions.push(session?);
        }

        Ok(sessions)
    }

    pub fn load_sessions_from_db(conn: &Connection) -> Result<Vec<Session>, Box<dyn Error>> {
        let mut stmt = conn.prepare(
            "SELECT name, files, pipelines, selected_files, query_mode FROM sessions"
        )?;

        let mut sessions = Vec::new();

        let rows = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let files_json: String = row.get(1)?;
            let pipelines_json: String = row.get(2)?;
            let selected_files_json: String = row.get(3)?;
            let query_mode_str: String = row.get(4)?;

            let files: Vec<String> = serde_json::from_str(&files_json)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e)
                ))?;

            let pipelines: Vec<Vec<String>> = serde_json::from_str(&pipelines_json)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e)
                ))?;

            let selected_files: Vec<usize> = serde_json::from_str(&selected_files_json)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e)
                ))?;

            let query_mode = DatabaseType::from_str(&query_mode_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(ErrorKind::InvalidData, "Empty CSV data"))
                ))?;

            Ok(Session {
                name,
                files,
                pipelines,
                selected_files,
                query_mode,
            })
        })?;

        for row in rows {
            sessions.push(row?);
        }

        Ok(sessions)
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

    pub fn load_file_from_db(conn: &mut Connection, session_name: &str)
        -> Result<Vec<(String, CsvGrid)>, Box<dyn Error>>
    {

        let mut stmt = conn.prepare(
            "SELECT file_path, file_content FROM files
         INNER JOIN sessions ON files.session_id = sessions.id
         WHERE sessions.name = ?"
        )?;

        let mut results = Vec::new();

        let rows = stmt.query_map([session_name], |row| {
            let file_path: String = row.get(0)?;
            let content: String = row.get(1)?;
            Ok((file_path, content))
        })?;

        for row in rows {
            if let Ok((file_path, content)) = row {
                let grid: CsvGrid = csv_parser(&content).expect("Failed to parse CSV content");
                results.push((file_path, grid));
            }
        }

        Ok(results)
    }

}