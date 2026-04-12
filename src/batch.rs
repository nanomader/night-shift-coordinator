use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::{NscError, Result};
use crate::slice::SliceCandidate;

pub fn discover_startup_candidates(slice_dir: &Path) -> Result<Vec<SliceCandidate>> {
    if !slice_dir.is_dir() {
        return Err(NscError::InvalidDirectory(slice_dir.to_path_buf()));
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(slice_dir).map_err(|source| NscError::PathIo {
        path: slice_dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| NscError::PathIo {
            path: slice_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if !is_markdown_file(&path) {
            continue;
        }
        candidates.push(validate_slice_file(path)?);
    }
    sort_candidates(&mut candidates);
    if candidates.is_empty() {
        return Err(NscError::NoSliceCandidates(slice_dir.to_path_buf()));
    }
    Ok(candidates)
}

fn validate_slice_file(path: PathBuf) -> Result<SliceCandidate> {
    let mut file = fs::File::open(&path).map_err(|source| NscError::PathIo {
        path: path.clone(),
        source,
    })?;
    let mut first_byte = [0u8; 1];
    let bytes_read = file
        .read(&mut first_byte)
        .map_err(|source| NscError::PathIo {
            path: path.clone(),
            source,
        })?;
    if bytes_read == 0 {
        return Err(NscError::EmptySliceFile(path));
    }
    candidate_from_path(path)
}

fn candidate_from_path(path: PathBuf) -> Result<SliceCandidate> {
    let label = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            NscError::Protocol(format!("slice file has no usable stem: {}", path.display()))
        })?;
    Ok(SliceCandidate { path, label })
}

fn is_markdown_file(path: &Path) -> bool {
    path.is_file() && path.extension().is_some_and(|ext| ext == "md")
}

fn sort_candidates(candidates: &mut [SliceCandidate]) {
    candidates.sort_by(|left, right| left.path.file_name().cmp(&right.path.file_name()));
}
