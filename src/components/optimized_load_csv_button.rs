use std::fs::File;
use std::io::{BufReader, BufRead};
use std::thread;
use std::sync::mpsc::Sender;
use rayon::prelude::*;
use egui::Ui;
use crate::charter_utilities::{combine_grids, csv_parser};

/// Represents the configuration for chunk processing
#[derive(Clone)]
struct ChunkConfig {
    chunk_size: usize,
    max_chunks_in_memory: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 8 * 1024 * 1024,
            max_chunks_in_memory: 4,
        }
    }
}

/// A reusable CSV file loader button component
pub struct CsvLoaderButton {
    pub config: ChunkConfig,
}

impl CsvLoaderButton {
    pub fn new() -> Self {
        Self {
            config: ChunkConfig::default(),
        }
    }

    pub fn with_config(config: ChunkConfig) -> Self {
        Self { config }
    }

    pub fn show(
        &self,
        ui: &mut Ui,
        file_sender: Sender<(String, Vec<Vec<String>>)>,
    ) {
        if ui.button("Load File").clicked() {
            self.handle_file_selection(file_sender);
        }
    }

    fn handle_file_selection(&self, file_sender: Sender<(String, Vec<Vec<String>>)>) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV files", &["csv"])
            .pick_file()
        {
            let path_as_string = path.to_str().unwrap().to_string();
            let sender = file_sender.clone();
            let config = self.config.clone();

            thread::spawn(move || {
                process_csv_file(&path_as_string, sender, &config);
            });
        }
    }
}

fn process_csv_file(
    path: &str,
    sender: Sender<(String, Vec<Vec<String>>)>,
    config: &ChunkConfig,
) {
    if let Ok(file) = File::open(path) {
        let mut reader = BufReader::new(file);
        let mut header = String::new();

        if reader.read_line(&mut header).is_ok() {
            header = header.trim().to_string();
            process_file_contents(&mut reader, &header, sender, path, config);
        }
    }
}

fn process_file_contents(
    reader: &mut BufReader<File>,
    header: &str,
    sender: Sender<(String, Vec<Vec<String>>)>,
    path: &str,
    config: &ChunkConfig,
) {
    let mut all_results: Vec<Vec<Vec<String>>> = Vec::new();
    let mut current_chunk = Vec::new();
    let mut current_chunk_size = 0;
    let mut chunks_to_process = Vec::new();

    for line in reader.lines() {
        if let Ok(line) = line {
            let line_len = line.len();
            current_chunk.push(line);
            current_chunk_size += line_len + 1;

            if current_chunk_size >= config.chunk_size {
                chunks_to_process.push(current_chunk);
                current_chunk = Vec::new();
                current_chunk_size = 0;

                if chunks_to_process.len() >= config.max_chunks_in_memory {
                    process_chunks(&mut all_results, &chunks_to_process, header);
                    chunks_to_process.clear();
                }
            }
        }
    }

    if !current_chunk.is_empty() {
        chunks_to_process.push(current_chunk);
    }
    if !chunks_to_process.is_empty() {
        process_chunks(&mut all_results, &chunks_to_process, header);
    }

    let combined_grid = combine_grids(all_results);
    let _ = sender.send((path.to_string(), combined_grid.rows));
}

fn process_chunks(
    all_results: &mut Vec<Vec<Vec<String>>>,
    chunks: &[Vec<String>],
    header: &str,
) {
    let results: Vec<_> = chunks
        .par_iter()
        .map(|chunk| {
            let chunk_content = format!("{}\n{}", header, chunk.join("\n"));
            csv_parser(&chunk_content).expect("Failed to parse chunk")
        })
        .collect();

    all_results.extend(results);
}