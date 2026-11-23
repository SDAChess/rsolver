//! Finding `Cargo.toml` in a gzipped tarball.

use std::io::{self, Read};
use std::path::{Component, Path};

// Crates are uploaded by anyone, so we should bound the maximum crate
// and manifest size.
const MAX_DECODED_SIZE: u64 = 1024 * 1024 * 1024;
const MAX_MANIFEST_SIZE: u64 = 4 * 1024 * 1024;

/// What decoding a possibly truncated gzipped tarball turned up.
pub(super) enum Scan {
    /// The whole `<dir>/Cargo.toml` entry.
    Manifest(Vec<u8>),
    /// The gzip stream decoded through its checksum and holds no manifest.
    Absent,
    /// Decoding failed: the prefix is too short, or the data is corrupt.
    Undecodable(io::Error),
    /// The named thing exceeds its size limit.
    TooLarge(&'static str),
}

/// Decodes `gz`, the start or the whole of a `.crate` file, as far as it goes
/// and looks for the crate's manifest.
pub(super) fn scan(gz: &[u8]) -> Scan {
    // Multi-member: the end of the first gzip member is not the end of the
    // data if another member follows.
    let decoder = flate2::bufread::MultiGzDecoder::new(gz).take(MAX_DECODED_SIZE);
    let mut archive = tar::Archive::new(decoder);
    let found = find_manifest(&mut archive);
    let mut rest = archive.into_inner();
    match found {
        Ok(Some(found)) => return found,
        Ok(None) => {}
        // Running into the limit looks like a truncated stream to tar.
        Err(_) if rest.limit() == 0 => return Scan::TooLarge("the unpacked crate"),
        Err(e) => return Scan::Undecodable(e),
    }
    // tar stops at its end marker; read on so that a stream cut short after it
    // fails its gzip checksum instead of passing for a tarball without manifest.
    match io::copy(&mut rest, &mut io::sink()) {
        Ok(_) if rest.limit() == 0 => Scan::TooLarge("the unpacked crate"),
        Ok(_) => Scan::Absent,
        Err(e) => Scan::Undecodable(e),
    }
}

/// The verdict on the manifest entry, or `None` if the archive ends without one.
fn find_manifest(archive: &mut tar::Archive<impl Read>) -> io::Result<Option<Scan>> {
    for entry in archive.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() || !is_manifest(&entry.path()?) {
            continue;
        }
        let size = entry.size();
        if size > MAX_MANIFEST_SIZE {
            return Ok(Some(Scan::TooLarge("Cargo.toml")));
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        // A stream that ends inside the entry must never yield a cut-off
        // manifest: what is returned here is stored forever.
        if bytes.len() as u64 != size {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        return Ok(Some(Scan::Manifest(bytes)));
    }
    Ok(None)
}

/// Whether `path` is `<dir>/Cargo.toml`, the manifest of the packaged crate.
fn is_manifest(path: &Path) -> bool {
    let mut components = path
        .components()
        .skip_while(|c| matches!(c, Component::CurDir));
    matches!(
        (components.next(), components.next(), components.next()),
        (Some(Component::Normal(_)), Some(Component::Normal(file)), None)
            if file == "Cargo.toml"
    )
}
