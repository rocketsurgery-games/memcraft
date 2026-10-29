//! `mcpe2java <pe world dir> <output>`
//!
//! Writes a Java 1.12.2 world. An output ending in `.zip` produces a zip
//! with `level.dat` at its root (loadable by drag-and-drop or `?map=<url>`
//! in the Minecraft Web Client); anything else is a world folder.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, output] = args.as_slice() else {
        eprintln!("usage: mcpe2java <pe world dir> <output dir | output.zip>");
        return ExitCode::from(2);
    };
    let world = match mcpe_world::World::open_dir(input) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: reading {input}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let java = mcpe2java::convert(&world);
    let output = PathBuf::from(output);
    let result = if output.extension().is_some_and(|e| e == "zip") {
        let parent = output.parent().filter(|p| !p.as_os_str().is_empty());
        parent
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::File::create(&output)).map_err(|e| e.to_string()).and_then(|f| java.write_zip(f).map_err(|e| e.to_string()))
    } else {
        java.write_dir(&output).map_err(|e| e.to_string())
    };
    if let Err(e) = result {
        eprintln!("error: writing {}: {e}", output.display());
        return ExitCode::FAILURE;
    }
    let r = &java.report;
    println!("converted {:?}: {} chunks, {} block entities", world.name().unwrap_or("?"), r.chunks, r.block_entities);
    if !r.skipped_block_entities.is_empty() {
        println!("skipped block entities: {:?}", r.skipped_block_entities);
    }
    println!("wrote {}", output.display());
    ExitCode::SUCCESS
}
