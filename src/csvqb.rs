use crate::charter_utilities::CsvGrid;

/// CIR is the Charting Intermediate Representation
#[derive(Debug, Clone)]
pub enum CIR {
    Bool(bool),
    Number(f64),
    Text(String),
    Field(String),
    QueryResult(Vec<Vec<String>>)
}

#[derive(Debug)]
pub enum Operator {
    Sum,
    Avg,
    Count,
    GroupBy,
    Equals,
    GreaterThan,
    LessThan,
}

pub fn col_sum(
    file_indexes: &Vec<usize>,
    csv_files: &Vec<(String, CsvGrid)>,
    column: &str,
    group_by: Option<&[String]>
) -> Vec<Vec<String>> {
    let mut result = std::collections::HashMap::new();
    let mut query_grid = Vec::new();
    for &file_idx in file_indexes {
        if let Some((_, grid)) = csv_files.get(file_idx) {
            if grid.is_empty() { continue; }

            // Find column index
            let headers = &grid[0];
            let col_idx = match headers.iter().position(|h| h == column) {
                Some(idx) => idx,
                None => continue,
            };

            // Process data rows
            for row in grid.iter().skip(1) {
                if row.len() <= col_idx { continue; }

                let key = match group_by {
                    Some(group_cols) => {
                        let mut key = String::new();
                        for group_col in group_cols {
                            if let Some(idx) = headers.iter().position(|h| h == group_col) {
                                if let Some(val) = row.get(idx) {
                                    key.push_str(val);
                                    key.push('|');
                                }
                            }
                        }
                        key
                    }
                    None => "column sum".to_string(),
                };

                if let Ok(value) = row[col_idx].parse::<f64>() {
                    *result.entry(key).or_insert(0.0) += value;
                }
            }
        }
        let mut header_row = Vec::new();
        if let Some(group_cols) = group_by {
            header_row.extend(group_cols.iter().cloned());
        } else {
            header_row.push(column.to_string());
        }

        header_row.push("sum".to_string());
        query_grid.push(header_row.clone());
        let _ = header_row.pop();

        for (key, count) in result.drain() {
            let mut row = Vec::new();
            if key.contains('|') {
                let values: Vec<&str> = key.split('|').filter(|s| !s.is_empty()).collect();
                row.extend(values.iter().map(|&s| s.to_string()));
            } else {
                row.push(key);
            }

            row.push(count.to_string());
            query_grid.push(row);
        }

    }

    query_grid
}
fn col_average(
    file_indexes: &Vec<usize>,
    csv_files: &Vec<(String, CsvGrid)>,
    column: &str,
    group_by: Option<&[String]>
) -> Vec<Vec<String>> {
    let mut sums = std::collections::HashMap::new();
    let mut counts = std::collections::HashMap::new();
    let mut query_grid = Vec::new();

    for &file_idx in file_indexes {
        if let Some((_, grid)) = csv_files.get(file_idx) {
            if grid.is_empty() { continue; }

            let headers = &grid[0];
            let col_idx = match headers.iter().position(|h| h == column) {
                Some(idx) => idx,
                None => continue,
            };

            for row in grid.iter().skip(1) {
                if row.len() <= col_idx { continue; }

                let key = match group_by {
                    Some(group_cols) => {
                        let mut key = String::new();
                        for group_col in group_cols {
                            if let Some(idx) = headers.iter().position(|h| h == group_col) {
                                if let Some(val) = row.get(idx) {
                                    key.push_str(val);
                                    key.push('|');
                                }
                            }
                        }
                        key
                    }
                    None => "average".to_string(),
                };

                if let Ok(value) = row[col_idx].parse::<f64>() {
                    *sums.entry(key.clone()).or_insert(0.0) += value;
                    *counts.entry(key).or_insert(0) += 1;
                }
            }
        }
    }

    sums = sums.into_iter()
        .filter_map(|(key, sum)| {
            counts.get(&key).map(|&count| {
                (key, sum / count as f64)
            })
        })
        .collect();

    let mut header_row = Vec::new();
    if let Some(group_cols) = group_by {
        header_row.extend(group_cols.iter().cloned());
    } else {
        header_row.push(column.to_string());
    }

    header_row.push("average".to_string());
    query_grid.push(header_row.clone());
    let _ = header_row.pop();

    for (key, count) in sums.drain() {
        let mut row = Vec::new();
        if key.contains('|') {
            let values: Vec<&str> = key.split('|').filter(|s| !s.is_empty()).collect();
            row.extend(values.iter().map(|&s| s.to_string()));
        } else {
            row.push(key);
        }

        row.push(count.to_string());
        query_grid.push(row);
    }
    query_grid
}

fn col_count(
    file_indexes: &Vec<usize>,
    csv_files: &Vec<(String, CsvGrid)>,
    column: &str,
    group_by: Option<&[String]>
) -> Vec<Vec<String>> {
    let mut counts = std::collections::HashMap::new();
    let mut query_grid = Vec::new();
    for &file_idx in file_indexes {
        if let Some((_, grid)) = csv_files.get(file_idx) {
            if grid.is_empty() { continue; }

            let headers = &grid[0];
            let col_idx = match headers.iter().position(|h| h == column) {
                Some(idx) => idx,
                None => continue,
            };

            for row in grid.iter().skip(1) {
                if row.len() <= col_idx { continue; }

                let key = match group_by {
                    Some(group_cols) => {
                        let mut key = String::new();
                        for group_col in group_cols {
                            if let Some(idx) = headers.iter().position(|h| h == group_col) {
                                if let Some(val) = row.get(idx) {
                                    key.push_str(val);
                                    key.push('|');
                                }
                            }
                        }
                        key
                    }
                    None => row[col_idx].clone(),
                };
                *counts.entry(key).or_insert(0) += 1;
            }

            let mut header_row = Vec::new();
            if let Some(group_cols) = group_by {
                header_row.extend(group_cols.iter().cloned());
            } else {
                header_row.push(column.to_string());
            }

            header_row.push("count".to_string());
            query_grid.push(header_row.clone());
            let _ = header_row.pop();

            for (key, count) in counts.drain() {
                let mut row = Vec::new();
                if key.contains('|') {
                    let values: Vec<&str> = key.split('|').filter(|s| !s.is_empty()).collect();
                    row.extend(values.iter().map(|&s| s.to_string()));
                } else {
                    row.push(key);
                }

                row.push(count.to_string());
                query_grid.push(row);
            }
        }
    }

    //counts.into_iter().collect()
    query_grid
}

fn filter_equals(
    file_indexes: &Vec<usize>,
    csv_files: &Vec<(String, CsvGrid)>,
    column: &str,
    value: &str
) -> Vec<Vec<String>> {
    let mut result = Vec::new();

    for &file_idx in file_indexes {
        if let Some((_, grid)) = csv_files.get(file_idx) {
            if grid.is_empty() { continue; }

            let headers = &grid[0];
            let col_idx = match headers.iter().position(|h| h == column) {
                Some(idx) => idx,
                None => continue,
            };

            result.push(headers.clone());
            for row in grid.iter().skip(1) {
                if row.len() > col_idx && row[col_idx] == value {
                    result.push(row.clone());
                }
            }
        }
    }

    result
}

fn filter_greater_than(
    file_indexes: &Vec<usize>,
    csv_files: &Vec<(String, CsvGrid)>,
    column: &str,
    value: f64
) -> Vec<Vec<String>> {
    let mut result = Vec::new();

    for &file_idx in file_indexes {
        if let Some((_, grid)) = csv_files.get(file_idx) {
            if grid.is_empty() { continue; }

            let headers = &grid[0];
            let col_idx = match headers.iter().position(|h| h == column) {
                Some(idx) => idx,
                None => continue,
            };

            result.push(headers.clone());
            for row in grid.iter().skip(1) {
                if row.len() > col_idx {
                    if let Ok(num) = row[col_idx].parse::<f64>() {
                        if num > value {
                            result.push(row.clone());
                        }
                    }
                }
            }
        }
    }

    result
}

pub fn process_csvqb_pipeline(
    qb_pipeline: &[String],
    file_indexes: &Vec<usize>,
    files: &Vec<(String, CsvGrid)>
) -> Vec<CIR> {
    let mut stack: Vec<CIR> = vec![];
    let mut results: Vec<CIR> = Vec::new();
    let mut capture_group: Vec<String> = Vec::new();
    let mut i = 0;

    while i < qb_pipeline.len() {
        match qb_pipeline[i].as_str() {
            "GRP" => {
                while i + 1 < qb_pipeline.len() {
                    if ["GRP", "CSUM", "CCOUNT", "CAVG", "MUL", "=", "<", ">"].contains(&qb_pipeline[i + 1].as_str()) {
                        break;
                    }
                    capture_group.push(qb_pipeline[i + 1].clone());
                    i+=1
                }
                i+=1
            }
            "CSUM" | "CCOUNT" | "CAVG" => {
                if i + 1 < qb_pipeline.len() {
                    let field = &qb_pipeline[i + 1];
                    let operation = qb_pipeline[i].as_str();

                    let filter_condition = if !capture_group.is_empty() {
                        Some(capture_group.clone())
                    } else {
                        None
                    };

                    let result = match operation {
                        "CSUM" => {
                            let sum = col_sum(file_indexes, files, field, filter_condition.as_deref());
                            CIR::QueryResult(sum)
                        }
                        "CCOUNT" => {
                            let counts = col_count(file_indexes, files, field, filter_condition.as_deref());
                            CIR::QueryResult(counts)
                        }
                        "CAVG" => {
                            let avg = col_average(file_indexes, files, field, filter_condition.as_deref());
                            CIR::QueryResult(avg)
                        }

                        _ => unreachable!()
                    };

                    results.push(result.clone());
                    stack.push(result);
                    i+=1
                }
            }
            "MUL" => {
                println!("stack: {:?}", stack);
                if let (Some(CIR::QueryResult(right)), Some(CIR::QueryResult(left))) = (stack.pop(), stack.pop()) {
                    if let (Ok(left_val), Ok(right_val)) = (left[1][1].parse::<f64>(), right[1][1].parse::<f64>()) {
                        stack.push(CIR::Number(left_val * right_val));
                        results.push(CIR::Number(left_val * right_val));
                        results.push(CIR::Field(left[0][0].clone() + " * ".as_str() + &right[0][0]));
                    }
                } else {
                    println!("err in MUL");
                    break;
                }
                i+=1;
            }

            ">" | "<" | "=" => {
                if stack.len() >= 2 {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    println!("left: {:?}, right: {:?}", left, right);
                    match qb_pipeline[i].as_str() {
                        ">" => {
                            let comparison = match (left, right) {
                                (CIR::Number(left), CIR::Number(right)) => {
                                    CIR::Bool(left > right)
                                }
                                _ => unreachable!()
                            };
                            results.push(comparison)
                        }
                        "<" => {
                            let comparison = match (left, right) {
                                (CIR::Number(left), CIR::Number(right)) => {
                                    CIR::Bool(left < right)
                                }
                                _ => unreachable!()
                            };
                            results.push(comparison)
                        }
                        "=" => {
                            let comparison = match (left, right) {
                                (CIR::Number(left), CIR::Number(right)) => {
                                    CIR::Bool(left == right)
                                }
                                _ => unreachable!()
                            };
                            results.push(comparison)
                        }
                        _ => unreachable!()
                    }
                }

                i+=1
            }
            "(" | ")" => {
                if qb_pipeline[i] == "(" {
                    while i < qb_pipeline.len() {
                        if qb_pipeline[i] == ")" {
                            break
                        }
                        let result = process_csvqb_pipeline(&qb_pipeline[i + 1..], file_indexes, files);
                        println!("result: {},  {:?}", i, result);
                        println!("stack: {},  {:?}", i, stack);
                        if !result.is_empty() {
                            results.push(result[0].clone());
                            break;
                        }
                        i+=1
                    }
                }
                i+=1
            }

            _ => {
                if let Ok(num) = qb_pipeline[i].parse::<f64>() {
                    stack.push(CIR::Number(num));
                }
                else {
                    results.push(CIR::Field(qb_pipeline[i].clone()));
                }
                i+=1
            }
        }
    }

    if results.is_empty() && !stack.is_empty() {
        results.push(stack.pop().unwrap());
    }

    results
}