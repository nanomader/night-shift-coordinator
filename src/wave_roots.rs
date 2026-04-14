use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{NscError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaveRoot {
    pub path: PathBuf,
    pub name: String,
    pub wave_id: u32,
}

pub fn discover_wave_roots(root: &Path) -> Result<Vec<WaveRoot>> {
    if !root.is_dir() {
        return Err(NscError::InvalidDirectory(root.to_path_buf()));
    }

    let mut wave_roots = Vec::new();
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
            return Err(NscError::InvalidProgramRootEntry {
                root: root.to_path_buf(),
                entry: path.display().to_string(),
            });
        };

        if is_allowed_non_wave_root_entry(&name) {
            continue;
        }
        if !path.is_dir() {
            return Err(NscError::InvalidProgramRootEntry {
                root: root.to_path_buf(),
                entry: name,
            });
        }

        let wave_id = parse_wave_id(&name).ok_or_else(|| NscError::InvalidWaveRootName {
            root: root.to_path_buf(),
            entry: name.clone(),
        })?;
        wave_roots.push(WaveRoot {
            path,
            name,
            wave_id,
        });
    }

    if wave_roots.is_empty() {
        return Err(NscError::NoWaveRoots(root.to_path_buf()));
    }

    wave_roots.sort_by_key(|wave_root| wave_root.wave_id);
    validate_contiguous_sequence(root, &wave_roots)?;
    Ok(wave_roots)
}

fn is_allowed_non_wave_root_entry(name: &str) -> bool {
    name == "README.md" || name.starts_with('.')
}

fn parse_wave_id(name: &str) -> Option<u32> {
    let suffix = name.strip_prefix("wave")?;
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

fn validate_contiguous_sequence(root: &Path, wave_roots: &[WaveRoot]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for wave_root in wave_roots {
        if !seen.insert(wave_root.wave_id) {
            return Err(NscError::DuplicateWaveRootId {
                root: root.to_path_buf(),
                wave_id: wave_root.wave_id,
            });
        }
    }

    let mut expected = wave_roots[0].wave_id;
    for wave_root in wave_roots {
        if wave_root.wave_id != expected {
            return Err(NscError::WaveRootGap {
                root: root.to_path_buf(),
                expected,
                actual: wave_root.wave_id,
            });
        }
        expected += 1;
    }
    Ok(())
}
