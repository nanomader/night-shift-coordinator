use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{NscError, Result};
use crate::slice::SliceCandidate;

#[derive(Debug, Default)]
pub struct RescanOutcome {
    pub new_candidates: Vec<SliceCandidate>,
    pub warnings: Vec<String>,
}

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
        let metadata = fs::metadata(&path).map_err(|source| NscError::PathIo {
            path: path.clone(),
            source,
        })?;
        if metadata.len() == 0 {
            return Err(NscError::EmptySliceFile(path));
        }
        candidates.push(candidate_from_path(path)?);
    }
    sort_candidates(&mut candidates);
    if candidates.is_empty() {
        return Err(NscError::NoSliceCandidates(slice_dir.to_path_buf()));
    }
    Ok(candidates)
}

pub fn discover_new_candidates_on_rescan(
    slice_dir: &Path,
    known_paths: &HashSet<PathBuf>,
) -> RescanOutcome {
    if !slice_dir.is_dir() {
        return RescanOutcome {
            warnings: vec![format!(
                "slice folder is not accessible during rescan: {}",
                slice_dir.display()
            )],
            ..RescanOutcome::default()
        };
    }

    let mut outcome = RescanOutcome::default();
    let read_dir = match fs::read_dir(slice_dir) {
        Ok(value) => value,
        Err(err) => {
            outcome.warnings.push(format!(
                "failed to rescan slice folder {}: {}",
                slice_dir.display(),
                err
            ));
            return outcome;
        }
    };

    for entry in read_dir {
        let entry = match entry {
            Ok(value) => value,
            Err(err) => {
                outcome
                    .warnings
                    .push(format!("failed to read slice-folder entry: {err}"));
                continue;
            }
        };
        let path = entry.path();
        if !is_markdown_file(&path) || known_paths.contains(&path) {
            continue;
        }
        match fs::metadata(&path) {
            Ok(metadata) if metadata.len() > 0 => match candidate_from_path(path.clone()) {
                Ok(candidate) => outcome.new_candidates.push(candidate),
                Err(err) => outcome.warnings.push(err.to_string()),
            },
            Ok(_) => outcome.warnings.push(format!(
                "ignoring empty slice file during rescan: {}",
                path.display()
            )),
            Err(err) => outcome.warnings.push(format!(
                "ignoring unreadable slice file during rescan: {} ({err})",
                path.display()
            )),
        }
    }
    sort_candidates(&mut outcome.new_candidates);
    outcome
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
