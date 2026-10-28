use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("Failed to parse {}", path.display()))
}

pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    std::fs::write(path, bytes).with_context(|| format!("Failed to write {}", path.display()))
}

pub fn hash_file(hash: &mut Sha256, path: &Path) -> Result<()> {
    let mut reader = BufReader::new(
        File::open(path).with_context(|| format!("Failed to open {}", path.display()))?,
    );
    loop {
        let bytes = reader
            .fill_buf()
            .with_context(|| format!("Failed to read {}", path.display()))?;
        if bytes.is_empty() {
            return Ok(());
        }
        hash.update(bytes);
        let length = bytes.len();
        reader.consume(length);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_files_across_buffer_boundaries() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("input");
        for bytes in [Vec::new(), vec![0xa5; 32769]] {
            std::fs::write(&path, &bytes).unwrap();
            let mut hash = Sha256::new();
            hash_file(&mut hash, &path).unwrap();
            assert_eq!(hash.finalize(), Sha256::digest(&bytes));
        }
    }
}
