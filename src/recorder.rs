use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Result, Write};
use std::path::Path;

use serde::Serialize;

pub struct Recorder {
    output: BufWriter<File>,
}

#[derive(Debug, Serialize)]
pub struct Record {
    pub operation: String,
    pub result: String,
}

impl Recorder {
    pub fn new<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        let output = BufWriter::new(file);
        Ok(Recorder { output })
    }

    pub fn record(&mut self, record: Record) -> Result<()> {
        serde_json::to_writer(&mut self.output, &record)?;
        self.output.write_all(b"\n")?;
        Ok(())
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.output.flush().unwrap();
    }
}
