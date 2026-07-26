use crate::errors::Result;
use crate::ingestion::pipeline::IngestionPipeline;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

pub struct LogStreamer {
    pipeline: Arc<IngestionPipeline>,
    running: Arc<AtomicBool>,
}

impl LogStreamer {
    pub fn new(pipeline: Arc<IngestionPipeline>) -> Self {
        Self {
            pipeline,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Tail a file and stream lines into the ingestion pipeline
    pub fn tail_file<P: AsRef<Path>>(&self, path: P, adapter_name: Option<String>) -> Result<()> {
        let path_buf = path.as_ref().to_path_buf();
        info!("Starting continuous stream tailing for: {:?}", path_buf);

        let file = File::open(&path_buf)?;
        let mut reader = BufReader::new(file);

        // Seek to end for streaming mode, or start at 0 if beginning
        reader.seek(SeekFrom::End(0))?;

        let running = self.running.clone();
        let pipeline = self.pipeline.clone();

        while running.load(Ordering::Relaxed) {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // EOF reached, wait briefly before checking for new data
                    thread::sleep(Duration::from_millis(500));
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        if let Err(e) = pipeline.process_str(trimmed, adapter_name.as_deref()) {
                            warn!("Error ingesting streamed log line: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Error reading line from streamed file: {}", e);
                    thread::sleep(Duration::from_millis(500));
                }
            }
        }

        info!("Log stream tailing stopped for: {:?}", path_buf);
        Ok(())
    }
}
