//! Opening Mojang's LevelDB fork.
//!
//! Mojang's fork differs from upstream LevelDB only in block compression:
//! id 2 is zlib (used by Pocket Edition), id 4 is raw deflate (later Bedrock).
//! We register both alongside the standard ones.
//!
//! The files are loaded into rusty-leveldb's in-memory environment, so
//! opening (which replays the write-ahead log and may compact) never touches
//! the original world.

use std::io::{Read, Write};
use std::path::Path;
use std::rc::Rc;

use rusty_leveldb::compressor::{NoneCompressor, SnappyCompressor};
use rusty_leveldb::env::Env;
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

/// Calls `f` with every live key/value pair, in key order.
pub fn for_each_entry(
    files: impl IntoIterator<Item = (String, Vec<u8>)>,
    mut f: impl FnMut(Vec<u8>, Vec<u8>),
) -> Result<()> {
    let env = MemEnv::new();
    let dir = Path::new("db");
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

    let opt = Options {
        env: Rc::new(Box::new(env)),
        compressor_list: Rc::new(compressors),
        create_if_missing: false,
        ..Options::default()
    };

    let mut db = DB::open(dir, opt)?;
    let mut it = db.new_iter()?;
    while let Some((k, v)) = it.next() {
        f(k.to_vec(), v.to_vec());
    }
    Ok(())
}
