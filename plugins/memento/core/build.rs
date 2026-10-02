use std::{env, fs, path::Path};

use sha2::{Digest, Sha256};

fn sources(path: &Path, files: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            sources(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=MEMENTO_BUILD_ID");
    let root = env::var("CARGO_MANIFEST_DIR")?;
    let root = Path::new(&root);
    let identity = match env::var("MEMENTO_BUILD_ID") {
        Ok(value) if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) => {
            value
        }
        Ok(_) => return Err("MEMENTO_BUILD_ID must be a SHA-256 hex digest".into()),
        Err(env::VarError::NotPresent) => {
            let mut files = vec![root.join("Cargo.toml"), root.join("build.rs")];
            sources(&root.join("src"), &mut files)?;
            files.sort();
            let mut digest = Sha256::new();
            for file in files {
                println!("cargo:rerun-if-changed={}", file.display());
                digest.update(file.strip_prefix(root)?.to_string_lossy().as_bytes());
                digest.update([0]);
                digest.update(fs::read(file)?);
                digest.update([0]);
            }
            format!("{:x}", digest.finalize())
        }
        Err(error) => return Err(error.into()),
    };
    println!("cargo:rustc-env=MEMENTO_BUILD_ID={identity}");
    Ok(())
}
