use std::path::Path;

use super::fetch;
use super::{Dependency, Manifest, PkgError};

/// `ny pkg init` — create a new ny.pkg manifest
pub fn cmd_init(project_dir: &Path) -> Result<(), PkgError> {
    let manifest_path = project_dir.join("ny.pkg");
    if manifest_path.exists() {
        return Err(PkgError::AlreadyExists(
            "ny.pkg already exists".to_string(),
        ));
    }

    let name = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-project")
        .to_string();

    let manifest = Manifest {
        name,
        version: "0.1.0".to_string(),
        dependencies: Vec::new(),
    };
    manifest.save(project_dir)?;

    // Add .ny_deps/ to .gitignore if it exists
    let gitignore = project_dir.join(".gitignore");
    if gitignore.exists() {
        let content = std::fs::read_to_string(&gitignore).unwrap_or_default();
        if !content.contains(".ny_deps") {
            std::fs::write(&gitignore, format!("{}.ny_deps/\n", content))?;
        }
    }

    eprintln!("created ny.pkg");
    Ok(())
}

/// `ny pkg add <url>` — add a dependency
pub fn cmd_add(
    start_dir: &Path,
    url: &str,
    name: Option<&str>,
    branch: Option<&str>,
) -> Result<(), PkgError> {
    fetch::check_git()?;

    let root = Manifest::find_project_root(start_dir)
        .ok_or_else(|| PkgError::NotFound("no ny.pkg found — run `ny pkg init` first".to_string()))?;

    let mut manifest = Manifest::load(&root)?;
    let pkg_name = name
        .map(|s| s.to_string())
        .unwrap_or_else(|| fetch::name_from_url(url));

    // Check for duplicate
    if manifest.dependencies.iter().any(|d| d.name == pkg_name) {
        return Err(PkgError::AlreadyExists(format!(
            "dependency '{}' already exists — use `ny pkg remove {}` first",
            pkg_name, pkg_name
        )));
    }

    let dep = Dependency {
        name: pkg_name.clone(),
        url: url.to_string(),
        branch: branch.map(|s| s.to_string()),
        sha: None,
    };

    let deps_dir = Manifest::deps_dir(&root);
    let sha = fetch::fetch_dependency(&dep, &deps_dir)?;

    manifest.dependencies.push(Dependency {
        sha: Some(sha.clone()),
        ..dep
    });
    manifest.save(&root)?;

    eprintln!("added {} @ {}", pkg_name, &sha[..8.min(sha.len())]);
    Ok(())
}

/// `ny pkg build` — fetch all dependencies
pub fn cmd_build(start_dir: &Path) -> Result<(), PkgError> {
    fetch::check_git()?;

    let root = Manifest::find_project_root(start_dir)
        .ok_or_else(|| PkgError::NotFound("no ny.pkg found — run `ny pkg init` first".to_string()))?;

    let mut manifest = Manifest::load(&root)?;
    let deps_dir = Manifest::deps_dir(&root);

    if manifest.dependencies.is_empty() {
        eprintln!("no dependencies");
        return Ok(());
    }

    let mut updated = false;
    for dep in &mut manifest.dependencies {
        let sha = fetch::fetch_dependency(dep, &deps_dir)?;
        if dep.sha.as_deref() != Some(&sha) {
            dep.sha = Some(sha.clone());
            updated = true;
            eprintln!("  fetched {} @ {}", dep.name, &sha[..8.min(sha.len())]);
        } else {
            eprintln!("  {} up to date", dep.name);
        }
    }

    if updated {
        manifest.save(&root)?;
    }
    eprintln!("{} dependencies ready", manifest.dependencies.len());
    Ok(())
}

/// `ny pkg remove <name>` — remove a dependency
pub fn cmd_remove(start_dir: &Path, name: &str) -> Result<(), PkgError> {
    let root = Manifest::find_project_root(start_dir)
        .ok_or_else(|| PkgError::NotFound("no ny.pkg found".to_string()))?;

    let mut manifest = Manifest::load(&root)?;
    let before = manifest.dependencies.len();
    manifest.dependencies.retain(|d| d.name != name);
    if manifest.dependencies.len() == before {
        return Err(PkgError::NotFound(format!(
            "dependency '{}' not found",
            name
        )));
    }

    // Remove directory
    let dep_dir = Manifest::deps_dir(&root).join(name);
    if dep_dir.exists() {
        std::fs::remove_dir_all(&dep_dir)?;
    }

    manifest.save(&root)?;
    eprintln!("removed {}", name);
    Ok(())
}

/// `ny pkg list` — list dependencies
pub fn cmd_list(start_dir: &Path) -> Result<(), PkgError> {
    let root = Manifest::find_project_root(start_dir)
        .ok_or_else(|| PkgError::NotFound("no ny.pkg found".to_string()))?;

    let manifest = Manifest::load(&root)?;
    println!("{} v{}", manifest.name, manifest.version);

    if manifest.dependencies.is_empty() {
        println!("  (no dependencies)");
    } else {
        for dep in &manifest.dependencies {
            let sha = dep.sha.as_deref().unwrap_or("unpinned");
            let short_sha = &sha[..8.min(sha.len())];
            println!("  {} @ {} ({})", dep.name, short_sha, dep.url);
        }
    }
    Ok(())
}
