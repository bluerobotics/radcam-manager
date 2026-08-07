use std::{fs, io::Write, path::Path};

use flate2::{Compression, write::GzEncoder};
use vergen_gix::{BuildBuilder, CargoBuilder, DependencyKind, GixBuilder};

/// Assets whose bytes are already compressed: deflating them again costs CPU on both
/// ends and saves nothing, so they are embedded verbatim.
const ALREADY_COMPRESSED: &[&str] = &[
    "woff2", "woff", "png", "jpg", "jpeg", "gif", "webp", "avif", "mp4", "zip", "gz",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    generate_build_details()?;
    embed_frontend_gzipped()?;

    Ok(())
}

/// Stage `frontend/dist` into `OUT_DIR` with every compressible asset gzipped, so the
/// binary carries the compressed bytes only and serves them without deflating per request.
fn embed_frontend_gzipped() -> Result<(), Box<dyn std::error::Error>> {
    let dist = Path::new(&std::env::var("CARGO_MANIFEST_DIR")?).join("../../../frontend/dist");
    let staged = Path::new(&std::env::var("OUT_DIR")?).join("dist_gz");

    println!("cargo:rerun-if-changed={}", dist.display());

    let _ = fs::remove_dir_all(&staged);
    fs::create_dir_all(&staged)?;

    if !dist.is_dir() {
        println!("cargo:warning=frontend/dist is missing; build the frontend first");
        return Ok(());
    }

    stage_gzipped(&dist, &staged)
}

fn stage_gzipped(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let nested = target.join(entry.file_name());
            fs::create_dir_all(&nested)?;
            stage_gzipped(&path, &nested)?;
            continue;
        }

        println!("cargo:rerun-if-changed={}", path.display());

        let name = entry.file_name();
        let extension = path.extension().and_then(|extension| extension.to_str());
        if extension.is_some_and(|extension| ALREADY_COMPRESSED.contains(&extension)) {
            fs::copy(&path, target.join(&name))?;
            continue;
        }

        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&fs::read(&path)?)?;
        fs::write(
            target.join(format!("{}.gz", name.to_string_lossy())),
            encoder.finish()?,
        )?;
    }

    Ok(())
}

fn generate_build_details() -> Result<(), Box<dyn std::error::Error>> {
    let mut emitter = vergen_gix::Emitter::default();

    emitter.add_instructions(&BuildBuilder::all_build()?)?;
    emitter.add_instructions(
        CargoBuilder::all_cargo()?.set_dep_kind_filter(Some(DependencyKind::Normal)),
    )?;

    if std::path::Path::new("../../.git").is_dir() {
        emitter.add_instructions(&GixBuilder::all_git()?)?;
    }

    emitter.emit()?;

    Ok(())
}
