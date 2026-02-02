use anyhow::Result;
use csv::ByteRecord;

pub struct CsvParser;

impl CsvParser {
    /// Parses a raw line string into a vector of fields.
    /// This is strict parsing; real world usage might need to handle malformed lines gracefully.
    pub fn parse_line(line: &str) -> Result<Vec<String>> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(line.as_bytes());

        let mut record = ByteRecord::new();
        if reader.read_byte_record(&mut record)? {
            let fields = record.iter()
                .map(|f| String::from_utf8_lossy(f).into_owned())
                .collect();
            Ok(fields)
        } else {
            // Empty line or parse error that resulted in no record
             Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let line = "a,b,c";
        let fields = CsvParser::parse_line(line).unwrap();
        assert_eq!(fields, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_quotes() {
        let line = "a,\"b,c\",d";
        let fields = CsvParser::parse_line(line).unwrap();
        assert_eq!(fields, vec!["a", "b,c", "d"]);
    }
}
