//! Opening Mojang's LevelDB fork.
//!
//! Mojang's fork differs from upstream LevelDB only in block compression:
//! id 2 is zlib (used by Pocket Edition), id 4 is raw deflate (later Bedrock).
//! We register both alongside the standard ones.
//!
//! The files are loaded into rusty-leveldb's in-memory environment, so
//! opening (which replays the write-ahead log and may compact) never touches
//! the original world. Saving reads the files back out of that environment.

use std::cell::Cell;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use rusty_leveldb::compressor::{NoneCompressor, SnappyCompressor};
use rusty_leveldb::env::{Env, FileLock, Logger, RandomAccess};
use rusty_leveldb::{Compressor, CompressorList, LdbIterator, MemEnv, Options, DB};

use crate::Result;

struct Zlib;

impl Compressor for Zlib {
    fn encode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
        let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        e.write_all(&block).and_then(|_| e.finish()).map_err(io_status)
    }

    fn decode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
        let mut out = Vec::new();
        flate2::read::ZlibDecoder::new(&block[..]).read_to_end(&mut out).map_err(io_status)?;
        Ok(out)
    }
}

struct RawDeflate;

impl Compressor for RawDeflate {
    fn encode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
        let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        e.write_all(&block).and_then(|_| e.finish()).map_err(io_status)
    }

    fn decode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
        let mut out = Vec::new();
        flate2::read::DeflateDecoder::new(&block[..]).read_to_end(&mut out).map_err(io_status)?;
        Ok(out)
    }
}

fn io_status(e: std::io::Error) -> rusty_leveldb::Status {
    rusty_leveldb::Status::new(rusty_leveldb::StatusCode::CompressionError, &e.to_string())
}

/// A [`MemEnv`] without a clock.
///
/// rusty-leveldb times its compactions with `SystemTime::now`, which panics on
/// `wasm32-unknown-unknown`. The timings only feed statistics, so a counter
/// stands in for the clock and the same code runs natively and in the browser.
struct ClocklessEnv {
    files: MemEnv,
    ticks: Cell<u64>,
}

impl Env for ClocklessEnv {
    fn open_sequential_file(&self, p: &Path) -> rusty_leveldb::Result<Box<dyn Read>> {
        self.files.open_sequential_file(p)
    }
    fn open_random_access_file(&self, p: &Path) -> rusty_leveldb::Result<Box<dyn RandomAccess>> {
        self.files.open_random_access_file(p)
    }
    fn open_writable_file(&self, p: &Path) -> rusty_leveldb::Result<Box<dyn Write>> {
        self.files.open_writable_file(p)
    }
    fn open_appendable_file(&self, p: &Path) -> rusty_leveldb::Result<Box<dyn Write>> {
        self.files.open_appendable_file(p)
    }
    fn exists(&self, p: &Path) -> rusty_leveldb::Result<bool> {
        self.files.exists(p)
    }
    fn children(&self, p: &Path) -> rusty_leveldb::Result<Vec<PathBuf>> {
        self.files.children(p)
    }
    fn size_of(&self, p: &Path) -> rusty_leveldb::Result<usize> {
        self.files.size_of(p)
    }
    fn delete(&self, p: &Path) -> rusty_leveldb::Result<()> {
        self.files.delete(p)
    }
    fn mkdir(&self, p: &Path) -> rusty_leveldb::Result<()> {
        self.files.mkdir(p)
    }
    fn rmdir(&self, p: &Path) -> rusty_leveldb::Result<()> {
        self.files.rmdir(p)
    }
    fn rename(&self, from: &Path, to: &Path) -> rusty_leveldb::Result<()> {
        self.files.rename(from, to)
    }
    fn lock(&self, p: &Path) -> rusty_leveldb::Result<FileLock> {
        self.files.lock(p)
    }
    fn unlock(&self, l: FileLock) -> rusty_leveldb::Result<()> {
        self.files.unlock(l)
    }
    fn new_logger(&self, p: &Path) -> rusty_leveldb::Result<Logger> {
        self.files.new_logger(p)
    }
    fn micros(&self) -> u64 {
        self.ticks.set(self.ticks.get() + 1);
        self.ticks.get()
    }
    fn sleep_for(&self, _micros: u32) {}
}

const DB_DIR: &str = "db";

/// An open world database and the in-memory files behind it.
pub struct MemDb {
    pub db: DB,
    env: Rc<Box<dyn Env>>,
}

impl MemDb {
    /// Opens the database made of `files` (the contents of a world's `db/`).
    pub fn open(files: impl IntoIterator<Item = (String, Vec<u8>)>) -> Result<MemDb> {
        let env = ClocklessEnv { files: MemEnv::new(), ticks: Cell::new(0) };
        let dir = Path::new(DB_DIR);
        env.mkdir(dir)?;
        for (name, bytes) in files {
            // LOCK is only a marker; MemEnv keeps its own lock table.
            if name == "LOCK" {
                continue;
            }
            env.open_writable_file(&dir.join(&name))?.write_all(&bytes)?;
        }

        let mut compressors = CompressorList::new();
        compressors.set(NoneCompressor);
        compressors.set(SnappyCompressor);
        compressors.set_with_id(2, Zlib);
        compressors.set_with_id(4, RawDeflate);

        let env: Rc<Box<dyn Env>> = Rc::new(Box::new(env));
        let opt = Options {
            env: env.clone(),
            compressor_list: Rc::new(compressors),
            // Tables written from here on are zlib-compressed, like the ones
            // Pocket Edition writes (and ones this reader already handles).
            compressor: 2,
            create_if_missing: false,
            ..Options::default()
        };
        let db = DB::open(dir, opt)?;
        Ok(MemDb { db, env })
    }

    /// Flushes the write-ahead log and returns every file of the database
    /// (except the `LOCK` marker), by name.
    pub fn close(mut self) -> Result<Vec<(String, Vec<u8>)>> {
        self.db.close()?;
        let dir = Path::new(DB_DIR);
        let mut files = Vec::new();
        for path in self.env.children(dir)? {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if name == "LOCK" {
                continue;
            }
            let mut bytes = Vec::new();
            self.env.open_sequential_file(&dir.join(&name))?.read_to_end(&mut bytes)?;
            files.push((name, bytes));
        }
        Ok(files)
    }
}

/// Calls `f` with every live key/value pair, in key order.
pub fn for_each_entry(
    files: impl IntoIterator<Item = (String, Vec<u8>)>,
    mut f: impl FnMut(Vec<u8>, Vec<u8>),
) -> Result<()> {
    let mut db = MemDb::open(files)?.db;
    let mut it = db.new_iter()?;
    while let Some((k, v)) = it.next() {
        f(k.to_vec(), v.to_vec());
    }
    Ok(())
}
