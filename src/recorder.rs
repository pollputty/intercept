use std::fs::OpenOptions;
use std::io::{BufWriter, Result, stdout, Write};

use serde::Serialize;

use crate::config::RecordConfig;
use crate::syscall::Clock;

pub struct Recorder {
    output: BufWriter<Box<dyn Write>>,
    config: RecordConfig,
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
pub struct TimeRecord {
    pub clock: Clock,
    pub time: Option<std::time::SystemTime>,
}

#[derive(Debug, Serialize)]
pub struct PIDRecord {
    pub pid: u32,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Record {
    File(FileRecord),
    Random(RandomRecord),
    Time(TimeRecord),
    PID(PIDRecord),
}

impl Recorder {
    pub fn new(cfg: &RecordConfig) -> Result<Self> {
        let file: Box<dyn Write> = match cfg.path.to_string_lossy().as_ref() {
            "-" => Box::new(stdout()),
            _ => Box::new(OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&cfg.path)?)
        };
        let output = BufWriter::new(file);
        Ok(Recorder {
            config: cfg.clone(),
            output,
        })
    }

    pub fn record(&mut self, record: Record) -> Result<()> {
        match record {
            Record::File(_) => {
                if !self.config.files {
                    return Ok(());
                }
            }
            Record::Random(_) => {
                if !self.config.random {
                    return Ok(());
                }
            }
            Record::Time(_) => {
                if !self.config.time {
                    return Ok(());
                }
            }
            Record::PID(_) => {
                if !self.config.pid {
                    return Ok(());
                }
            }
        };
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

impl From<TimeRecord> for Record {
    fn from(record: TimeRecord) -> Self {
        Record::Time(record)
    }
}

impl From<PIDRecord> for Record {
    fn from(record: PIDRecord) -> Self {
        Record::PID(record)
    }
}