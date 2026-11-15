//! Platform factory: all UI services are asynchronous, never blocking HTTP.
use crate::backend::async_client::Client;
#[derive(Clone)]
pub struct Services {
    #[cfg(not(target_arch = "wasm32"))]
    native: crate::backend::Handle,
    #[cfg(target_arch = "wasm32")]
    endpoint: String,
    #[cfg(target_arch = "wasm32")]
    limits: bool,
}
impl Services {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn native(native: crate::backend::Handle) -> Self {
        Self { native }
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn native_handle(&self) -> &crate::backend::Handle {
        &self.native
    }
    #[cfg(target_arch = "wasm32")]
    pub fn browser(endpoint: String, limits: bool) -> Self {
        Self { endpoint, limits }
    }
    pub fn client(&self) -> Client {
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::backend::native_transport::NativeTransport::client(self.native.clone())
        }
        #[cfg(target_arch = "wasm32")]
        {
            // Correlation namespace only, not an authentication credential.
            let session = crate::web::client_id();
            Client::new(
                session,
                Box::new(
                    crate::backend::web_transport::WebTransport::new(
                        &self.endpoint,
                        if self.limits { 120_000 } else { 0 },
                    )
                    .expect("Validated same-origin endpoint")
                    .with_limits(self.limits),
                ),
            )
            .expect("Nonzero session")
            .with_limits(self.limits)
        }
    }
}

/// Hosting policy is independent of the shared native/browser editor logic.
pub fn browser_mode() -> bool {
    #[cfg(target_arch = "wasm32")]
    { crate::web::browser_mode() }
    #[cfg(not(target_arch = "wasm32"))]
    { false }
}
