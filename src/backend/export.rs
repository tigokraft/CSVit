use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

pub fn export_to_json(input_path: &str, output_path: &str) -> Result<()> {
    let input = File::open(input_path)?;
    let mut reader = csv::ReaderBuilder::new().from_reader(BufReader::new(input));
    
    let output = File::create(output_path)?;
    let mut writer = BufWriter::new(output);

    let headers = reader.headers()?.clone();
    
    writer.write_all(b"[")?;

    let mut first = true;
    for result in reader.records() {
        let record = result?;
        
        if !first {
            writer.write_all(b",")?;
        }
        first = false;

        let mut map = serde_json::Map::new();
        for (i, field) in record.iter().enumerate() {
            let key = headers.get(i).unwrap_or(&format!("Col {}", i)).to_string();
            map.insert(key, serde_json::Value::String(field.to_string()));
        }

        serde_json::to_writer(&mut writer, &map)?;
    }

    writer.write_all(b"]")?;
    writer.flush()?;

    Ok(())
}
