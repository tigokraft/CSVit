use anyhow::{Context, Result};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

pub struct CsvLoader {
    mmap: Arc<Mmap>,
    /// Start byte offset of each record
    record_offsets: Vec<u64>,
    /// Total number of records (rows)
    total_records: usize,
}

impl CsvLoader {
    pub fn new(path: &Path) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("Failed to open file: {:?}", path))?;
        // Safety: We assume the file is not modified by other processes while we read.
        // For a text editor, this is a standard risk we accept, or we'd lock it (but O/S locks vary).
        let mmap = unsafe { Mmap::map(&file).context("Failed to memory map file")? };
        let mmap = Arc::new(mmap);

        let offsets = Self::build_index(&mmap)?;

        Ok(Self {
            record_offsets: offsets.clone(),
            total_records: offsets.len(),
            mmap,
        })
    }

    /// Scans the file to find the start of every record, respecting quotes.
    fn build_index(data: &[u8]) -> Result<Vec<u64>> {
        let mut offsets = Vec::new();
        if data.is_empty() {
            return Ok(offsets);
        }

        // The first record always starts at 0
        offsets.push(0);

        let mut in_quote = false;
        let mut i = 0;
        let len = data.len();

        while i < len {
            let b = data[i];
            
            match b {
                b'"' => {
                    in_quote = !in_quote;
                }
                b'\n' => {
                    if !in_quote {
                        // Found a record separator
                        if i + 1 < len {
                            offsets.push((i + 1) as u64);
                        }
                    }
                }
                b'\r' => {
                    // Handle CRLF: If \r\n, we wait for the \n.
                    // If just \r (classic Mac), we treat as newline if not in quote?
                    // Modern CSV usually expects \n or \r\n. 
                    // We'll ignore \r for the purpose of triggering a line break, 
                    // relying on the following \n. 
                    // Edge case: Old Mac files (\r only). 
                    // Let's assume standard \n or \r\n for now.
                }
                _ => {}
            }
            i += 1;
        }

        Ok(offsets)
    }

    pub fn get_record_line(&self, index: usize) -> Option<&[u8]> {
        if index >= self.record_offsets.len() {
            return None;
        }

        let start = self.record_offsets[index] as usize;
        let end = if index + 1 < self.record_offsets.len() {
            // End is the start of next line - 1 (to exclude newline potentially? No, include it to keep raw)
            // Actually, we usually want the raw bytes of the line including the newline chars for editing fidelity?
            // Or just the content?
            // Let's return the slice up to the next record start.
            // But wait, the next record start includes the previous newline?
            // our logic: offsets push (i+1). So i was the \n.
            // So [start .. next_start] includes the \n at the end of the line.
            self.record_offsets[index + 1] as usize
        } else {
            self.mmap.len()
        };

        if start >= self.mmap.len() || start >= end {
            // Empty last line or error
            return None;
        }

        Some(&self.mmap[start..end])
    }
    
    pub fn total_records(&self) -> usize {
        self.total_records
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_indexer_simple() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        write!(file, "a,b,c\n1,2,3\n4,5,6")?;
        
        let loader = CsvLoader::new(file.path())?;
        assert_eq!(loader.total_records(), 3);
        
        // Line 0: "a,b,c\n"
        let line0 = std::str::from_utf8(loader.get_record_line(0).unwrap())?;
        assert_eq!(line0, "a,b,c\n");

        // Line 2: "4,5,6" (no newline at EOF)
        let line2 = std::str::from_utf8(loader.get_record_line(2).unwrap())?;
        assert_eq!(line2, "4,5,6");

        Ok(())
    }

    #[test]
    fn test_indexer_quoted_newlines() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        write!(file, "a,b,\"c\nd\"\n1,2,3")?;
        
        let loader = CsvLoader::new(file.path())?;
        assert_eq!(loader.total_records(), 2);
        
        // Line 0: "a,b,\"c\nd\"\n"
        let line0 = std::str::from_utf8(loader.get_record_line(0).unwrap())?;
        assert_eq!(line0, "a,b,\"c\nd\"\n");

        // Line 1: "1,2,3"
        let line1 = std::str::from_utf8(loader.get_record_line(1).unwrap())?;
        assert_eq!(line1, "1,2,3");

        Ok(())
    }
}
