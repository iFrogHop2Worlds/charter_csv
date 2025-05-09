use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Write, BufRead, BufReader, ErrorKind};
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;
use itertools::Itertools;
use rusqlite::{params, Connection};
use serde_json::Value;
use crate::charter_utilities::{csv_parser, CsvGrid};
use crate::db_manager::DatabaseType;

#[derive(Debug, Clone)]
pub struct Session {
    pub(crate) name: String,
    pub(crate) files: Vec<String>,
    pub(crate) pipelines: Vec<Vec<String>>,
    pub(crate) selected_files: Vec<usize>,
    pub(crate) query_mode: DatabaseType,
}

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub(crate) name: String,
    pub(crate) file_count: usize,
    pub(crate) pipeline_count: usize,
    pub(crate) query_mode: String,
}

impl Session {
    pub fn new(name: String, files: Vec<String>, pipelines: Vec<Vec<String>>) -> Self {
        Self {
            name,
            files,
            pipelines,
            selected_files: vec![],
            query_mode: DatabaseType::CsvQB,
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn csv_files(&self) -> Vec<String> {
        let mut csv_files = Vec::new();
        for file in self.files.clone() {
            csv_files.push(file);
        }
        csv_files
    }

    pub fn csvqb_pipelines(&self) -> Vec<String> {
        let mut csvqb_pipelines = Vec::new();
        for pipeline in self.files.clone() {
            println!("{:?}", pipeline);
            csvqb_pipelines.push(pipeline);
        }
        csvqb_pipelines
    }
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
        println!("{:?}", pipelines_json);
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

        let raw_pipelines: Vec<Vec<String>> = serde_json::from_str(&pipelines_json)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(e)
            ))?;

        let mut pipelines: Vec<Vec<String>> = Vec::new();
        let mut current_group: Vec<String> = Vec::new();
        let mut last_num = 0;

        for query_group in raw_pipelines.iter().flat_map(|group| group.iter()) {
            if let Some(num_char) = query_group.chars().next() {
                if let Ok(num) = num_char.to_string().parse::<i32>() {
                    if num == last_num {
                        if !query_group.is_empty() {
                            current_group.push(query_group.to_string());
                        }
                    } else  {
                        if !current_group.is_empty() {
                            pipelines.push(current_group.clone());
                            current_group = Vec::new();
                            current_group.push(query_group.to_string());
                        }
                        last_num += 1;
                    }
                }
            }
        }

        if !current_group.is_empty() {
            pipelines.push(current_group);
        }

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
                Box::new(io::Error::new(ErrorKind::InvalidData, "Empty CSV data"))
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

pub fn update_current_session(
    csv_files: &Vec<(String, CsvGrid)>,
    csvqb_pipelines: &mut Vec<Vec<(usize, Vec<String>)>>,
    sessions: &mut Vec<Session>,
    multi_pipeline_tracker: &mut HashMap<usize, Vec<usize>>,
    current_session: usize,
    query_mode: &DatabaseType,
    conn: Connection
) {
    let mut file_paths: Vec<String> = vec![];
    let mut pipelines: Vec<Vec<String>> = vec![];

    for (path, _) in csv_files.iter() {
        file_paths.push(path.to_string());
    }

    for (_index, pipeline) in csvqb_pipelines.iter_mut().enumerate() {
        for (pipeline_number, query_string) in pipeline.iter_mut() {
            let mut pipeline_collection = vec![];
            let pipeline_str = format!("{} {}", pipeline_number.to_string(), query_string.join(" "));
            pipeline_collection.push(pipeline_str);
            pipelines.push(pipeline_collection);
        }
    }

    let ssi = current_session;
    let selected_files = multi_pipeline_tracker.keys().copied().sorted().collect();

    let session = Session {
        name: sessions[ssi].name.to_string(),
        files: file_paths,
        pipelines,
        selected_files,
        query_mode: query_mode.clone(),
    };

    if let Err(err) = save_session_to_database(conn, vec![session]) {
        println!("{}", format!("Error saving session to sql lite db: {}", err));
    }
}
/// Deprecated: May be used in future redundancy
pub fn save_session(
    session_name: String,
    csv_files: Vec<String>,
    pipelines: Vec<String>,
    selected_files: Vec<usize>,
    query_mode: &DatabaseType
) -> io::Result<()> {
    let sessions_dir = Path::new("C:/source/Charter_CSV/src/sessions");
    if !sessions_dir.exists() {
        fs::create_dir_all(sessions_dir)?;
    }

    let file_name = Path::new(&session_name).file_name().unwrap();
    let full_path = sessions_dir.join(file_name);
    let mut file = File::create(&full_path)?;
    
    for file_path in csv_files.clone() {
        writeln!(file, "{}", file_path)?;
    }
    
    writeln!(file)?;
    
    for row in pipelines.clone() {
        writeln!(file, "{}", row)?;
    }

    writeln!(file)?;
    
    for index in selected_files {
        writeln!(file, "{}", index)?;
    }

    writeln!(file)?;

    writeln!(file, "{}", query_mode)?;

    Ok(())
}
/// Deprecated: May be used in future redundancy
pub fn load_sessions_from_directory() -> io::Result<Vec<Session>> {
    let mut sessions = Vec::new();
    let sessions_dir = Path::new("C:/source/Charter_CSV/src/sessions");

    if sessions_dir.exists() && sessions_dir.is_dir() {
        for entry in fs::read_dir(sessions_dir)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.is_file() {
                let file_name = file_path
                    .file_stem()
                    .and_then(|os_str| os_str.to_str())
                    .unwrap_or("")
                    .to_string();

                let file = File::open(&file_path)?;
                let reader = BufReader::new(file);
                let mut csvqb_pipeline = vec![];
                let mut csv_files = vec![];
                let mut selected_files = vec![];
                let mut query_mode = DatabaseType::CsvQB;
                let mut current_section = 0;
                for line in reader.lines() {
                    let line = line?;
                    if line.trim().is_empty() {
                        current_section += 1;
                        continue;
                    }
                    match current_section {
                        0 => csv_files.push(line),
                        1 => csvqb_pipeline.push(line.trim_start().trim_start_matches(|c: char| c.is_numeric()).split_whitespace().map(|s| s.to_string()).collect()),
                        2 => {
                            if let Ok(index) = line.parse::<usize>() {
                                selected_files.push(index);
                            }
                        }
                        3 => {
                            if let Ok(db_type) = DatabaseType::from_str(&line) {
                                query_mode = db_type;
                            }
                        }
                        _ => break,
                    }
                }

                sessions.push(Session {
                    name: file_name,
                    files: csv_files,
                    pipelines: csvqb_pipeline,
                    selected_files,
                    query_mode 
                });
            }
        }
    }

    Ok(sessions)
}

pub fn load_session_files_from_db(conn: &mut    Connection, session_name: &str)
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

pub fn reconstruct_session(
    session: Session,
) -> mpsc::Receiver<(String, CsvGrid)> {
    let (sender, receiver) = mpsc::channel();
    let files = session.files.clone();

    thread::spawn(move || {
        for file_path in files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                let grid: CsvGrid = csv_parser(&content).expect("Failed to load file in session reconstruction");
                let _ = sender.send((file_path, grid));
            }
        }
    });

    receiver
}

