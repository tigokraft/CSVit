use anyhow::Result;
use std::cmp::min;
use std::sync::Arc;

use super::loader::CsvLoader;

pub struct PagedReader {
    loader: Arc<CsvLoader>,
    page_size: usize,
}

impl PagedReader {
    pub fn new(loader: Arc<CsvLoader>) -> Self {
        Self {
            loader,
            page_size: 100,
        }
    }

    pub fn empty() -> Self {
        Self {
            loader: Arc::new(CsvLoader::empty(0, 0)),
            page_size: 100,
        }
    }

    pub fn set_page_size(&mut self, size: usize) {
        self.page_size = size;
    }

    /// Returns a vector of raw string slices for the requested rows.
    /// Range is [start, start + len).
    pub fn get_rows(&self, start: usize, len: usize) -> Result<Vec<String>> {
        let mut rows = Vec::with_capacity(len);
        let total = self.loader.total_records();
        let end = min(start + len, total);

        for i in start..end {
            if let Some(bytes) = self.loader.get_record_line(i) {
                // We do a lossy utf8 conversion here for display purposes.
                // In a real editor we might want to keep bytes if encoding is weird,
                // but for now String is fine.
                let line = String::from_utf8_lossy(bytes).into_owned();
                rows.push(line);
            } else {
                break;
            }
        }

        Ok(rows)
    }
}
