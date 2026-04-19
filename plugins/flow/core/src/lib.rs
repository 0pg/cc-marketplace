pub mod dag;

use std::path::Path;

pub use dag::{validate, DagFile, Report, ValidationError};

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {path} as dag.json: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

pub fn validate_dag_file(path: &Path) -> Result<Report, LoadError> {
    let bytes = std::fs::read(path).map_err(|e| LoadError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let dag: DagFile = serde_json::from_slice(&bytes).map_err(|e| LoadError::Parse {
        path: path.display().to_string(),
        source: e,
    })?;
    Ok(validate(&dag))
}
