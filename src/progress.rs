use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

const CHUNK: usize = 1 << 20;

#[derive(Debug, Default)]
pub struct Progress {
    done: AtomicU64,
    total: AtomicU64,
}

impl Progress {
    pub fn start(&self, total: u64) {
        self.total.store(total, Relaxed);
        self.done.store(0, Relaxed);
    }

    pub fn expect(&self, more: u64) {
        self.total.fetch_add(more, Relaxed);
    }

    pub fn add(&self, bytes: u64) {
        self.done.fetch_add(bytes, Relaxed);
    }

    pub fn clear(&self) {
        self.total.store(0, Relaxed);
        self.done.store(0, Relaxed);
    }

    pub fn done(&self) -> u64 {
        self.done.load(Relaxed)
    }

    pub fn total(&self) -> u64 {
        self.total.load(Relaxed)
    }

    pub fn fraction(&self) -> Option<f32> {
        let total = self.total();
        if total == 0 {
            return None;
        }

        Some((self.done() as f32 / total as f32).clamp(0.0, 1.0))
    }
}

pub fn copy(from: &Path, to: &Path, progress: &Progress) -> io::Result<()> {
    let mut source = fs::File::open(from)?;
    let mut sink = fs::File::create(to)?;
    let mut buffer = vec![0u8; CHUNK];

    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        sink.write_all(&buffer[..read])?;
        progress.add(read as u64);
    }

    sink.flush()
}

pub struct Counting<'a, W> {
    pub inner: W,
    pub progress: &'a Progress,
}

impl<W: Write> Write for Counting<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.progress.add(written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
#[path = "tests/progress.rs"]
mod tests;
