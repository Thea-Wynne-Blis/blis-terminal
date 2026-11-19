use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use serde::Serialize;
use tempfile::NamedTempFile;

pub(crate) fn write_json(path: &Path, data: &impl Serialize) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("missing config directory"))?;
    fs::create_dir_all(parent)?;
    let mut file = NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut file, data)?;
    file.write_all(b"\n")?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}
