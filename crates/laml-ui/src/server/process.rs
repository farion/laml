//! Framed subprocess IPC. No request can leave a compiler running past its deadline.
use crate::backend::protocol::{self, Response, codec};
use std::{path::PathBuf, process::Stdio};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
};
pub struct Process {
    workspace: PathBuf,
    recovery: PathBuf,
    child: Option<Child>,
    limits: bool,
}
impl Process {
    pub fn with_limits(workspace: PathBuf, recovery: PathBuf, limits: bool) -> Self {
        Self {
            workspace,
            recovery,
            child: None,
            limits,
        }
    }
    async fn exchange(&mut self, input: &[u8]) -> Result<Vec<u8>, String> {
        if self.child.is_none() {
            // Cargo may replace the binary while this host is still running.
            // On Linux current_exe then names a deleted file; /proc/self/exe
            // keeps the running executable available for worker startup.
            #[cfg(target_os = "linux")]
            let executable = PathBuf::from("/proc/self/exe");
            #[cfg(not(target_os = "linux"))]
            let executable = std::env::current_exe()
                .map_err(|_| "Host executable unavailable")?;
            let mut command = Command::new(executable);
            command
                .arg("--host-worker")
                .arg(&self.workspace)
                .arg(&self.recovery);
            if self.limits {
                command.arg("--limits");
            }
            let child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .map_err(|error| format!("Could not start compiler worker: {error}"))?;
            self.child = Some(child);
        }
        let child = self.child.as_mut().unwrap();
        let length =
            u32::try_from(input.len()).map_err(|_| "IPC frame exceeds representable length")?;
        let input_stream = child.stdin.as_mut().ok_or("Compiler input unavailable")?;
        input_stream
            .write_all(&length.to_be_bytes())
            .await
            .map_err(|_| "Compiler stopped")?;
        input_stream
            .write_all(input)
            .await
            .map_err(|_| "Compiler stopped")?;
        input_stream.flush().await.map_err(|_| "Compiler stopped")?;
        let output = child.stdout.as_mut().ok_or("Compiler output unavailable")?;
        let length = output
            .read_u32()
            .await
            .map_err(|_| "Compiler stopped or resource limit reached")?
            as usize;
        if self.limits && length > protocol::MAX_RESPONSE_BYTES {
            return Err("Compiler response too large".into());
        }
        let mut bytes = vec![0; length];
        output
            .read_exact(&mut bytes)
            .await
            .map_err(|_| "Compiler response interrupted")?;
        let _: Response =
            codec::decode_response(&bytes).map_err(|_| "Invalid compiler response")?;
        Ok(bytes)
    }
    pub async fn request(&mut self, input: &[u8]) -> Result<Vec<u8>, String> {
        let result = if self.limits {
            tokio::time::timeout(std::time::Duration::from_secs(40), self.exchange(input))
                .await
                .unwrap_or_else(|_| {
                    Err("Compiler deadline exceeded; source remains in the editor".into())
                })
        } else {
            self.exchange(input).await
        };
        if result.is_err()
            && let Some(mut child) = self.child.take()
        {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        result
    }
}
