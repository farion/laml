//! Compiler frontend: correlation/transport are asynchronous; commands remain
//! frontend-owned and only commit after a matching, verified compiler response.
use super::{BuildResult, Command, Job, verify};
use laml_ui::backend::{
    async_client,
    protocol::{self, Failure, Operation, Reply, Stamp},
};
use std::sync::mpsc::TryRecvError;

pub struct Client {
    pub presentation: laml_ui::backend::Presentation,
    document: u64,
    service: async_client::Client,
    current: Option<(Stamp, Option<Command>)>,
    retryable: bool,
}
impl Client {
    pub fn with_client(document: u64, service: async_client::Client) -> Self {
        Self {
            presentation: laml_ui::backend::Presentation::Document,
            document,
            service,
            current: None,
            retryable: false,
        }
    }
    pub fn send(&mut self, job: Job) -> Result<(), Failure> {
        let (revision, source, path, command) = job;
        let stamp = self.service.submit(
            self.document,
            revision,
            Operation::Compile {
                source,
                address: protocol::address(&path)?,
                presentation: self.presentation,
            },
        )?;
        if let Some((previous, _)) = self.current.replace((stamp, command)) {
            self.service.forget(previous);
        }
        Ok(())
    }
    pub fn busy(&self) -> bool {
        self.current.is_some() && !self.service.failed()
    }
    pub fn forget_current(&mut self) {
        if let Some((stamp, _)) = self.current.take() {
            self.service.forget(stamp);
        }
    }
    pub fn take_retryable(&mut self) -> bool {
        std::mem::take(&mut self.retryable)
    }
    pub fn try_recv(&mut self) -> Result<BuildResult, TryRecvError> {
        let Some(response) = self.service.poll() else {
            return Err(if self.service.failed() {
                TryRecvError::Disconnected
            } else {
                TryRecvError::Empty
            });
        };
        let Some((stamp, command)) = self.current.take() else {
            return Err(TryRecvError::Empty);
        };
        debug_assert_eq!(stamp, response.stamp);
        self.retryable = !self.service.failed()
            && response.result.as_ref().err().is_some_and(|e| {
                matches!(
                    e.code,
                    protocol::FailureCode::Disconnected
                        | protocol::FailureCode::Busy
                        | protocol::FailureCode::Timeout
                )
            });
        let result = match response.result {
            Ok(Reply::Compiled(result)) => result.map(|preview| *preview),
            Err(error) => Err(error.to_string().into()),
            _ => Err("Invalid compiler response".into()),
        }
        .and_then(|preview| {
            if let Some(command) = &command {
                verify(&preview, &command.expectation, self.presentation)?;
            }
            Ok(preview)
        });
        Ok((stamp.revision, result, command))
    }
}
