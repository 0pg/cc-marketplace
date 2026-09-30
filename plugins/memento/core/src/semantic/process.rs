use super::{
    EmbeddingProvider, EmbeddingRequest, EmbeddingResponse, RerankProvider, RerankRequest,
    RerankResponse, SemanticError,
};
use serde::{Serialize, de::DeserializeOwned};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub(super) struct LocalCommand<'a> {
    pub command: &'a [String],
}

impl EmbeddingProvider for LocalCommand<'_> {
    fn embed(
        &self,
        request: &EmbeddingRequest,
        timeout_ms: u64,
    ) -> Result<EmbeddingResponse, SemanticError> {
        self.invoke(request, timeout_ms)
    }
}

impl RerankProvider for LocalCommand<'_> {
    fn score(
        &self,
        request: &RerankRequest,
        timeout_ms: u64,
    ) -> Result<RerankResponse, SemanticError> {
        self.invoke(request, timeout_ms)
    }
}

impl LocalCommand<'_> {
    fn invoke<T: DeserializeOwned>(
        &self,
        request: &impl Serialize,
        timeout_ms: u64,
    ) -> Result<T, SemanticError> {
        let program = self.command.first().ok_or_else(|| {
            SemanticError::Unavailable("no local semantic command configured".into())
        })?;
        let args = self.command.get(1..).unwrap_or_default();
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env("HF_HUB_OFFLINE", "1")
            .env("TRANSFORMERS_OFFLINE", "1")
            .spawn()
            .map_err(|e| SemanticError::Unavailable(e.to_string()))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| SemanticError::Unavailable("no backend stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SemanticError::Unavailable("no backend stdout".into()))?;
        let input = serde_json::to_vec(request)?;
        let (sender, receiver) = mpsc::channel();
        let (writer_sender, writer_receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = stdin.write_all(&input);
            drop(stdin);
            let _ = writer_sender.send(result);
        });
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            // Bound an erroneous executable's output independently of model input.
            let result = stdout
                .take(64 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map(|_| bytes);
            let _ = sender.send(result);
        });
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    return Err(SemanticError::Unavailable(format!(
                        "local semantic command exited {status}; inspect its local model setup"
                    )));
                }
                let remaining = Duration::from_millis(timeout_ms).saturating_sub(started.elapsed());
                let output = receiver.recv_timeout(remaining).map_err(|_| {
                    SemanticError::BudgetExceeded("backend output deadline".into())
                })??;
                if output.len() > 64 * 1024 * 1024 {
                    return Err(SemanticError::BudgetExceeded("backend output bytes".into()));
                }
                let remaining = Duration::from_millis(timeout_ms).saturating_sub(started.elapsed());
                writer_receiver.recv_timeout(remaining).map_err(|_| {
                    SemanticError::BudgetExceeded("backend input deadline".into())
                })??;
                return serde_json::from_slice(&output)
                    .map_err(|e| SemanticError::InvalidResponse(e.to_string()));
            }
            if started.elapsed() >= Duration::from_millis(timeout_ms) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(SemanticError::BudgetExceeded(
                    "local semantic command deadline".into(),
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
