use std::path::Path;
use std::process::Command;

use super::{Dependency, PkgError};

/// Check that git is available.
pub fn check_git() -> Result<(), PkgError> {
    Command::new("git")
        .arg("--version")
        .output()
        .map_err(|_| PkgError::GitFailed("git not found — install git to use ny pkg".to_string()))?;
    Ok(())
}

/// Clone a git repository (depth=1 for speed).
pub fn git_clone(url: &str, dest: &Path, branch: Option<&str>) -> Result<(), PkgError> {
    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--depth=1");
    if let Some(b) = branch {
        cmd.arg("--branch").arg(b);
    }
    cmd.arg(url).arg(dest);

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PkgError::GitFailed(stderr.trim().to_string()));
    }
    Ok(())
}

/// Get the HEAD SHA of a git repository.
pub fn git_head_sha(repo: &Path) -> Result<String, PkgError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("rev-parse")
        .arg("HEAD")
        .output()?;
    if !output.status.success() {
        return Err(PkgError::GitFailed("failed to get HEAD SHA".to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Fetch a dependency. Returns the SHA of the fetched commit.
/// Idempotent: skips if dest exists and SHA matches.
pub fn fetch_dependency(dep: &Dependency, deps_dir: &Path) -> Result<String, PkgError> {
    let dest = deps_dir.join(&dep.name);

    if dest.exists() {
        if let Some(expected_sha) = &dep.sha {
            if let Ok(current_sha) = git_head_sha(&dest) {
                if &current_sha == expected_sha {
                    return Ok(current_sha); // Already at correct version
                }
            }
        }
        // Exists but wrong SHA or no SHA — re-clone
        std::fs::remove_dir_all(&dest)?;
    }

    // Clone
    std::fs::create_dir_all(deps_dir)?;
    git_clone(&dep.url, &dest, dep.branch.as_deref())?;
    git_head_sha(&dest)
}

/// Derive package name from a git URL.
/// "https://github.com/user/math-extra.git" → "math-extra"
pub fn name_from_url(url: &str) -> String {
    let name = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("package");
    name.strip_suffix(".git").unwrap_or(name).to_string()
}
