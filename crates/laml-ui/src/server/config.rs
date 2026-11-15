use std::{
    ffi::OsString,
    net::SocketAddr,
    path::{Path, PathBuf},
};

pub struct Config {
    pub workspace: PathBuf,
    pub assets: PathBuf,
    pub state: PathBuf,
    pub bind: SocketAddr,
    pub origin: Option<String>,
    pub limits: bool,
    pub parallelism: Option<usize>,
    pub http_checks: bool,
    pub require_https: bool,
}
impl Config {
    pub fn parse(args: &[OsString]) -> Result<Self, String> {
        let (mut workspace, mut assets, mut state) = (None, None, None);
        let mut bind = "127.0.0.1:7878".parse().unwrap();
        let (mut origin, mut parallelism) = (None, None);
        let (mut limits, mut http_checks, mut require_https) = (false, false, false);
        let mut args = args.iter();
        while let Some(key) = args.next() {
            let key = key.to_str().ok_or("Invalid host option")?;
            match key {
                "--limits" => {
                    limits = true;
                    continue;
                }
                "--http-checks" => {
                    http_checks = true;
                    continue;
                }
                "--require-https" => {
                    require_https = true;
                    continue;
                }
                _ => {}
            }
            let value = args
                .next()
                .ok_or_else(|| format!("Missing value for {key}"))?;
            match key {
                "--workspace" => workspace = Some(PathBuf::from(value)),
                "--assets" => assets = Some(PathBuf::from(value)),
                "--state-dir" => state = Some(PathBuf::from(value)),
                "--bind" => {
                    bind = value
                        .to_str()
                        .ok_or("Invalid bind address")?
                        .parse()
                        .map_err(|_| "Invalid bind address")?
                }
                "--origin" => {
                    origin = Some(value.to_str().ok_or("Invalid public origin")?.to_owned())
                }
                "--parallelism" => {
                    let count = value
                        .to_str()
                        .ok_or("Invalid parallelism")?
                        .parse::<usize>()
                        .map_err(|_| "Parallelism must be a positive integer")?;
                    if count == 0 || count > tokio::sync::Semaphore::MAX_PERMITS {
                        return Err("Parallelism is outside the supported positive range".into());
                    }
                    parallelism = Some(count);
                }
                _ => return Err(format!("Unknown host option: {key}")),
            }
        }
        let workspace = std::fs::canonicalize(workspace.ok_or("Host requires --workspace DIR")?)
            .map_err(|_| "Workspace does not exist")?;
        if workspace.parent().is_none() {
            return Err("Choose a workspace, not a filesystem root".into());
        }
        let assets = std::fs::canonicalize(assets.ok_or("Host requires --assets DIR")?)
            .map_err(|_| "Web assets directory does not exist")?;
        let state = state.ok_or("Host requires --state-dir DIR outside the workspace")?;
        private_dir(&state)?;
        let state =
            std::fs::canonicalize(state).map_err(|_| "Could not resolve host state directory")?;
        if state.starts_with(&workspace)
            || workspace.starts_with(&state)
            || state.starts_with(&assets)
        {
            return Err("Keep host state outside workspace and public assets".into());
        }
        Ok(Self {
            workspace,
            assets,
            state,
            bind,
            origin,
            limits,
            parallelism,
            http_checks,
            require_https,
        })
    }
    pub fn public_origin(&self, local: SocketAddr) -> Result<String, String> {
        let origin = self
            .origin
            .clone()
            .unwrap_or_else(|| format!("http://{local}"));
        let uri: axum::http::Uri = origin.parse().map_err(|_| "Invalid public origin")?;
        if !matches!(uri.scheme_str(), Some("http" | "https"))
            || uri.authority().is_none()
            || uri.path_and_query().is_some_and(|p| p.as_str() != "/")
            || origin.ends_with('/')
            || origin.contains(['@', '?', '#'])
        {
            return Err("Origin must be an http(s) origin without path or credentials".into());
        }
        if self.require_https && uri.scheme_str() != Some("https") {
            return Err(
                "--require-https requires an HTTPS --origin and a TLS reverse proxy".into(),
            );
        }
        Ok(origin.to_ascii_lowercase())
    }
}
pub fn private_dir(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|_| "Could not create host state directory")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "Could not protect host state directory")?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policies_are_disabled_by_default_and_enabled_only_explicitly() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["workspace", "assets"] {
            std::fs::create_dir(dir.path().join(name)).unwrap();
        }
        let args = vec![
            OsString::from("--workspace"),
            dir.path().join("workspace").into(),
            "--assets".into(),
            dir.path().join("assets").into(),
            "--state-dir".into(),
            dir.path().join("state").into(),
            "--bind".into(),
            "0.0.0.0:7878".into(),
        ];
        let plain = Config::parse(&args).unwrap();
        assert!(!plain.limits && !plain.http_checks && !plain.require_https);
        assert!(plain.parallelism.is_none());
        assert!(
            plain
                .public_origin(plain.bind)
                .unwrap()
                .starts_with("http:")
        );
        let mut enabled = args;
        enabled.extend(
            [
                "--limits",
                "--http-checks",
                "--require-https",
                "--parallelism",
                "3",
            ]
            .map(OsString::from),
        );
        let configured = Config::parse(&enabled).unwrap();
        assert!(configured.limits && configured.http_checks && configured.require_https);
        assert_eq!(configured.parallelism, Some(3));
        assert!(configured.public_origin(configured.bind).is_err());
    }
}
