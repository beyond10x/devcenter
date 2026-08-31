//! Confidential-marker scanner used by source and release gates.

use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

/// One marker occurrence, carrying only its location and marker ordinal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Scanned path.
    pub path: PathBuf,
    /// One-based line number; zero means the path itself matched.
    pub line: usize,
    /// One-based marker position in the supplied private denylist.
    pub marker: usize,
}

/// Read non-empty, non-comment markers from a private runtime file.
pub fn read_markers(path: &Path) -> Result<Vec<String>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("cannot read denylist {}", path.display()))?;
    let markers: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect();
    if markers.is_empty() {
        bail!("denylist {} has no markers", path.display());
    }
    Ok(markers)
}

/// Scan path names and UTF-8 file content without printing the confidential marker itself.
pub fn scan(root: &Path, markers: &[String]) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(included)
    {
        let entry = entry.with_context(|| format!("cannot walk {}", root.display()))?;
        let relative = entry.path().strip_prefix(root).unwrap_or(entry.path());
        let rendered = relative.to_string_lossy().to_ascii_lowercase();
        for (index, marker) in markers.iter().enumerate() {
            if rendered.contains(&marker.to_ascii_lowercase()) {
                findings.push(Finding {
                    path: relative.to_owned(),
                    line: 0,
                    marker: index + 1,
                });
            }
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(bytes) = fs::read(entry.path()) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(&bytes) else {
            continue;
        };
        for (line_index, line) in text.lines().enumerate() {
            let lowercase = line.to_ascii_lowercase();
            for (index, marker) in markers.iter().enumerate() {
                if lowercase.contains(&marker.to_ascii_lowercase()) {
                    findings.push(Finding {
                        path: relative.to_owned(),
                        line: line_index + 1,
                        marker: index + 1,
                    });
                }
            }
        }
    }
    Ok(findings)
}

fn included(entry: &DirEntry) -> bool {
    !matches!(
        entry.file_name().to_str(),
        Some(".git" | "target" | "node_modules" | ".devcenter")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_location_without_echoing_marker() {
        let directory = tempfile::tempdir().expect("temp directory");
        fs::write(
            directory.path().join("document.txt"),
            "safe\nPrivateMarker\n",
        )
        .expect("fixture");
        let findings = scan(directory.path(), &["privatemarker".into()]).expect("scan");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 2);
        assert_eq!(findings[0].marker, 1);
    }
}
