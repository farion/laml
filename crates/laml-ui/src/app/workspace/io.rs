use super::*;
use laml_ui::backend::protocol::{self, Operation, Reply};

impl Workspace {
    pub(super) fn open_path(&mut self, path: PathBuf) {
        if self.opening.is_some() || self.closing_all {
            return;
        }
        let request = protocol::address(&path).and_then(|address| {
            self.service
                .submit(u64::MAX, 0, Operation::OpenSource { address })
        });
        match request {
            Ok(stamp) => {
                self.opening = Some(stamp);
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }
    pub(super) fn poll_open(&mut self) {
        while let Some(response) = self.service.poll() {
            if self.opening != Some(response.stamp) {
                continue;
            }
            self.opening = None;
            if self.closing_all || self.allow_close {
                continue;
            }
            match response.result {
                Ok(Reply::OpenedSource { address, source }) => {
                    let path = PathBuf::from(address);
                    if let Some(index) = self
                        .tabs
                        .iter()
                        .position(|tab| tab.file.path.as_ref() == Some(&path))
                    {
                        self.active = index;
                        self.reveal_active = true;
                    } else {
                        self.add_tab(Editor::with_services(
                            source,
                            Some(path),
                            self.backend.clone(),
                        ));
                    }
                }
                Err(error) => self.error = Some(error.to_string()),
                _ => self.error = Some("Unexpected open-file response".into()),
            }
        }
    }
}
