use std::fs::{self, File};
use std::io::{self, Write, BufRead, BufReader};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use crate::charter_utilities::{csv2grid, CsvGrid};

#[derive(Debug, Clone)]
pub struct Session {
    pub(crate) name: String,
    pub(crate) files: Vec<String>,
    pub(crate) pipelines: Vec<Vec<String>>,
    pub(crate) selected_files: Vec<usize>,
}

impl Session {
    pub fn new(name: String, files: Vec<String>, pipelines: Vec<Vec<String>>) -> Self {
        Self {
            name,
            files,
            pipelines,
            selected_files: vec![],
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
pub fn save_session(session_name: String, csv_files: Vec<String>, pipelines: Vec<String>, selected_files: Vec<usize>) -> io::Result<()> {
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

    Ok(())
}
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

                let mut current_section = 0;
                for line in reader.lines() {
                    let line = line?;
                    if line.trim().is_empty() {
                        current_section += 1;
                        continue;
                    }
                    match current_section {
                        0 => csv_files.push(line),
                        1 => csvqb_pipeline.push(line.trim_start().split_whitespace().map(|s| s.to_string()).collect()),
                        2 => {
                            if let Ok(index) = line.parse::<usize>() {
                                selected_files.push(index);
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
                });
            }
        }
    }

    Ok(sessions)
}

pub fn reconstruct_session(
    session: Session,
) -> mpsc::Receiver<(String, CsvGrid)> {
    let (sender, receiver) = mpsc::channel();
    let files = session.files.clone();

    thread::spawn(move || {
        for file_path in files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                let grid: CsvGrid = csv2grid(&content);
                let _ = sender.send((file_path, grid));
            }
        }
    });

    receiver
}

