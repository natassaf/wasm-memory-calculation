
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::fs::File;


pub fn append_data_to_file(data:&str, file_path:&str) -> Result<(), std::io::Error> {
    let mut file = OpenOptions::new().create(true).append(true)
    .open(file_path)?;
    file.write_all(data.as_bytes())?;
    file.write_all(b"\n")?; // Add newline character
    Ok(())
}