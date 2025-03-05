use std::fs::{self, File};
use std::io::{self, Write, BufRead, BufReader};
use std::path::Path;
#[derive(Debug)]
pub struct Session {
    pub(crate) name: String,
    pub(crate) data: Vec<(String, Vec<Vec<String>>)>,
}

impl Session {
    pub fn new(name: String, data: Vec<(String, Vec<Vec<String>>)>) -> Self {
        Self {
            name,
            data
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn csv_files(&self) -> Vec<String> {
        let mut csv_files = Vec::new();
        for session in self.data.clone() {
            csv_files.push(session.0);
        }
        csv_files
    }

    pub fn csvqb_pipelines(&self) -> Vec<Vec<Vec<String>>> {
        let mut csvqb_pipelines = Vec::new();
        for session in self.data.clone() {
            println!("{:?}", session.1);
            csvqb_pipelines.push(session.1);
        }
        csvqb_pipelines
    }
}
pub fn save_session(csv_files: Vec<String>, pipelines: Vec<String>) -> io::Result<()> {
    let sessions_dir = Path::new("C:/source/Charter_CSV/src/sessions");
    if !sessions_dir.exists() {
        fs::create_dir_all(sessions_dir)?;
    }

    for file_path in &csv_files {
        let file_name = Path::new(file_path).file_name().unwrap();
        let full_path = sessions_dir.join(file_name);
        let mut file = File::create(&full_path)?;
        for row in pipelines.clone() {
            writeln!(file, "{}", row)?;
        }
    }

    Ok(())
}
pub fn restore_sessions() -> io::Result<Vec<Session>> {
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
                let mut csvqb_pipeline = Vec::new();

                for line in reader.lines() {
                    let line = line?;
                    let row: Vec<String> = line.split(' ').map(String::from).collect();
                    csvqb_pipeline.push(row);
                }

                sessions.push(Session {
                    name: file_name,  //Should be a session identifier
                    data: vec![(file_path.to_string_lossy().to_string(), csvqb_pipeline)],
                });

                // todo: Reconstruct a default session, add support to change sessions.
            }
        }
    }

    Ok(sessions)
}