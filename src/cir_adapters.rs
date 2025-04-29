use std::path::PathBuf;
use crate::csvqb::CIR;

pub fn sqlite_cir_adapter(combined_query: String, conn_path: PathBuf, graph_data: &mut Vec<Vec<CIR>>) {
    if let Ok(mut conn) = rusqlite::Connection::open(conn_path) {
        let (graph_type, query) = if combined_query.starts_with("Bar Graph") {
            ("Bar Graph", combined_query["Bar Graph".len()..].trim_start())
        } else if combined_query.starts_with("Histogram") {
            ("Histogram", combined_query["Histogram".len()..].trim_start())
        } else if combined_query.starts_with("Pie Chart") {
            ("Pie Chart", combined_query["Pie Chart".len()..].trim_start())
        } else if combined_query.starts_with("Scatter Plot") {
            ("Scatter Plot", combined_query["Scatter Plot".len()..].trim_start())
        } else if combined_query.starts_with("Line Chart") {
            ("Line Chart", combined_query["Line Chart".len()..].trim_start())
        } else if combined_query.starts_with("Flame Graph") {
            ("Flame Graph", combined_query["Flame Graph".len()..].trim_start())
        } else {
            ("", combined_query.as_str())
        };

        let mut stmt = match conn.prepare(query) {
            Ok(stmt) => stmt,
            Err(e) => {
                eprintln!("Error preparing SQL statement: {}", e);
                return;
            }
        };

        let column_names: Vec<String> = stmt.column_names()
            .iter()
            .map(|&name| name.to_string())
            .collect();

        let mut result_rows: Vec<Vec<String>> = vec![column_names];
        let column_count = stmt.column_count();

        let rows = match stmt.query_map([], |row| {
            let mut row_data = Vec::new();
            for i in 0..column_count {
                let value = match row.get_ref(i)? {
                    rusqlite::types::ValueRef::Null => "NULL".to_string(),
                    rusqlite::types::ValueRef::Integer(i) => i.to_string(),
                    rusqlite::types::ValueRef::Real(f) => f.to_string(),
                    rusqlite::types::ValueRef::Text(t) => String::from_utf8_lossy(t).to_string(),
                    rusqlite::types::ValueRef::Blob(_) => "[BLOB]".to_string(),
                };
                row_data.push(value);
            }
            Ok(row_data)
        }) {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("Error executing query: {}", e);
                return;
            }
        };

        for row_result in rows {
            match row_result {
                Ok(row) => result_rows.push(row),
                Err(e) => {
                    eprintln!("Error reading row: {}", e);
                    continue;
                }
            }
        }

        if !result_rows.is_empty() {
            let mut results = Vec::new();
            results.push(CIR::Field(graph_type.to_string()));
            results.push(CIR::QueryResult(result_rows));
            graph_data.push(results);
        }

    }
}