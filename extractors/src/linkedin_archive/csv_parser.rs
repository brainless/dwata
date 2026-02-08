use csv::ReaderBuilder;
use serde::de::DeserializeOwned;
use shared_types::ExtractionError;
use std::collections::HashMap;

pub struct CsvParser {
    delimiter: u8,
    has_headers: bool,
}

impl CsvParser {
    pub fn new() -> Self {
        Self {
            delimiter: b',',
            has_headers: true,
        }
    }

    pub fn parse_file<T: DeserializeOwned>(
        &self,
        content: &[u8],
    ) -> Result<Vec<T>, ExtractionError> {
        let mut reader = ReaderBuilder::new()
            .delimiter(self.delimiter)
            .has_headers(self.has_headers)
            .from_reader(content);

        let mut records = Vec::new();

        for result in reader.deserialize() {
            match result {
                Ok(record) => records.push(record),
                Err(e) => {
                    eprintln!("Failed to parse CSV row: {}", e);
                }
            }
        }

        Ok(records)
    }

    pub fn parse_to_maps(
        &self,
        content: &[u8],
    ) -> Result<Vec<HashMap<String, String>>, ExtractionError> {
        let mut reader = ReaderBuilder::new()
            .delimiter(self.delimiter)
            .has_headers(self.has_headers)
            .from_reader(content);

        let headers = reader
            .headers()
            .map_err(|e| ExtractionError::ParseError(e.to_string()))?
            .clone();

        let mut records = Vec::new();

        for result in reader.records() {
            match result {
                Ok(record) => {
                    let mut map = HashMap::new();
                    for (i, field) in record.iter().enumerate() {
                        if let Some(header) = headers.get(i) {
                            map.insert(header.to_string(), field.to_string());
                        }
                    }
                    records.push(map);
                }
                Err(e) => {
                    eprintln!("Failed to parse CSV row: {}", e);
                }
            }
        }

        Ok(records)
    }
}
