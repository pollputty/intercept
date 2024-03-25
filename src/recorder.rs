use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Result, Write};
use std::path::Path;

use serde::Serialize;

pub struct Recorder {
    output: BufWriter<File>,
}

#[derive(Debug, Serialize)]
pub struct FileRecord {
    pub path: String,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct RandomRecord {
    pub length: usize,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Record {
    File(FileRecord),
    Random(RandomRecord),
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

    pub fn record<T>(&mut self, record: T) -> Result<()>
    where
        T: Into<Record>,
    {
        serde_json::to_writer(&mut self.output, &record.into())?;
        self.output.write_all(b"\n")?;
        Ok(())
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.output.flush().unwrap();
    }
}

impl From<FileRecord> for Record {
    fn from(record: FileRecord) -> Self {
        Record::File(record)
    }
}

impl From<RandomRecord> for Record {
    fn from(record: RandomRecord) -> Self {
        Record::Random(record)
    }
}
