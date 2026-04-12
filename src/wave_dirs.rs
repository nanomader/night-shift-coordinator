use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{NscError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaveDir {
    pub path: PathBuf,
    pub name: String,
    pub batch_id: u32,
}

pub fn discover_wave_dirs(root: &Path) -> Result<Vec<WaveDir>> {
    if !root.is_dir() {
        return Err(NscError::InvalidDirectory(root.to_path_buf()));
    }

    let mut wave_dirs = Vec::new();
    for entry in fs::read_dir(root).map_err(|source| NscError::PathIo {
        path: root.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| NscError::PathIo {
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let Some(name) = path
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_owned)
        else {
            return Err(NscError::InvalidWaveRootEntry {
                root: root.to_path_buf(),
                entry: path.display().to_string(),
            });
        };

        if is_allowed_non_wave_entry(&name) {
            continue;
        }
        if !path.is_dir() {
            return Err(NscError::InvalidWaveRootEntry {
                root: root.to_path_buf(),
                entry: name,
            });
        }

        let batch_id = parse_batch_id(&name).ok_or_else(|| NscError::InvalidWaveDirName {
            root: root.to_path_buf(),
            entry: name.clone(),
        })?;
        wave_dirs.push(WaveDir {
            path,
            name,
            batch_id,
        });
    }

    if wave_dirs.is_empty() {
        return Err(NscError::NoWaveDirs(root.to_path_buf()));
    }

    wave_dirs.sort_by_key(|wave| wave.batch_id);
    validate_contiguous_sequence(root, &wave_dirs)?;
    Ok(wave_dirs)
}

fn is_allowed_non_wave_entry(name: &str) -> bool {
    name == "README.md" || name.starts_with('.')
}

fn parse_batch_id(name: &str) -> Option<u32> {
    let suffix = name.strip_prefix("batch")?;
    let (digits, tail) = suffix.split_once('_')?;
    if digits.len() != 2 || !digits.chars().all(|ch| ch.is_ascii_digit()) || tail.is_empty() {
        return None;
    }
    if !tail
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-'))
    {
        return None;
    }
    digits.parse().ok()
}

fn validate_contiguous_sequence(root: &Path, wave_dirs: &[WaveDir]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for wave in wave_dirs {
        if !seen.insert(wave.batch_id) {
            return Err(NscError::DuplicateWaveDirBatchId {
                root: root.to_path_buf(),
                batch_id: wave.batch_id,
            });
        }
    }

    let mut expected = wave_dirs[0].batch_id;
    for wave in wave_dirs {
        if wave.batch_id != expected {
            return Err(NscError::WaveDirGap {
                root: root.to_path_buf(),
                expected,
                actual: wave.batch_id,
            });
        }
        expected += 1;
    }
    Ok(())
}
