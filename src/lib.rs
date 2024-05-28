use std::{
    env,
    fs::read_to_string,
    io,
    path::{Path, PathBuf},
    process::Command,
    string::FromUtf8Error,
};
use thiserror::Error;
use zbus::blocking::fdo::PeerProxy;
use zbus::blocking::Connection;

#[derive(Error, Debug)]
pub enum UnsandboxError {
    #[error("IO: `{0}`")]
    IO(#[from] io::Error),
    #[error("The program is not sandboxed.")]
    NotSandboxed,
    #[error("LD not found.")]
    LdNotFound,
    #[error("Failed to convert utf8 string `{0}`")]
    FailedFromUtf8(#[from] FromUtf8Error),
    #[error("Failed to read config")]
    ConfigReadError,
    #[error("Zbus: `{0}`")]
    ZbusError(#[from] zbus::Error),
    #[error("No --talk-name=org.freedesktop.Flatpak permission for this Flatpak")]
    NoPermissions,
}

#[derive(Clone, Debug)]
pub enum CmdArg {
    StringArg(String),
    PathArg(PathBuf),
    PathDelimArg(Vec<PathBuf>, String),
}

impl CmdArg {
    pub fn new_path<P: AsRef<Path>>(p: P) -> Self {
        Self::PathArg(p.as_ref().into())
    }

    pub fn new_string(s: String) -> Self {
        Self::StringArg(s.into())
    }

    pub fn new_guess(s: String) -> Self {
        if Path::new(&s).exists() {
            Self::PathArg(s.into())
        } else {
            for delim in [":", ","] {
                let x = s
                    .split(delim)
                    .map(|p| Path::new(p).to_path_buf())
                    .collect::<Vec<_>>();
                let mut all_exists = true;
                for pth in x.clone() {
                    if !pth.exists() {
                        all_exists = false;
                        break;
                    }
                }
                if all_exists {
                    return Self::PathDelimArg(x, delim.into());
                }
            }
            Self::StringArg(s)
        }
    }

    fn into_string(&self, flatpak: FlatpakInfo) -> String {
        match self {
            Self::PathArg(pth) => flatpak.to_host_path(pth).to_string_lossy().to_string(),
            Self::StringArg(s) => s.clone(),
            Self::PathDelimArg(p, delim) => p
                .iter()
                .map(|x| flatpak.to_host_path(x).to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(delim),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FlatpakInfo {
    app_path: PathBuf,
    runtime_path: PathBuf,
}

impl FlatpakInfo {
    pub fn new() -> Result<FlatpakInfo, UnsandboxError> {
        if !is_flatpaked() {
            log::error!("This instance is not sandboxed!");
            return Err(UnsandboxError::NotSandboxed);
        } else if !has_flatpak_spawn_permission().is_ok_and(|x| x) {
            log::error!(
                "This instance does not have the --talk-name=org.freedesktop.Flatpak permission!"
            );
            return Err(UnsandboxError::NoPermissions);
        }
        log::info!("Sandbox passed checks!");
        let mut config = configparser::ini::Ini::new();
        if let Err(_) = config.read(read_to_string("/.flatpak-info")?) {
            log::error!("Could not read flatpak-info config");
            return Err(UnsandboxError::ConfigReadError);
        }
        let app_path = Path::new(&config.get("Instance", "app-path").unwrap()).to_path_buf();
        let runtime_path =
            Path::new(&config.get("Instance", "runtime-path").unwrap()).to_path_buf();
        log::debug!(
            "app_path: {}, runtime_path: {}",
            &app_path.to_string_lossy(),
            &runtime_path.to_string_lossy()
        );
        Ok(FlatpakInfo {
            app_path,
            runtime_path,
        })
    }

    pub fn to_host_path(&self, path: impl Into<PathBuf>) -> PathBuf {
        let path: PathBuf = path.into();
        if path.starts_with("/app") {
            self.app_path.join(path.strip_prefix("/app").unwrap())
        } else if path.starts_with("/usr") {
            self.runtime_path.join(path.strip_prefix("/usr").unwrap())
        } else {
            path
        }
    }

    pub fn get_ld_path(&self) -> Result<PathBuf, UnsandboxError> {
        let out = Command::new("ldconfig").arg("-p").output()?;
        for l in String::from_utf8(out.stdout)?.lines() {
            if l.trim().starts_with("ld-linux") {
                return Ok(self.to_host_path(l.split(" => ").nth(1).unwrap().trim()));
            }
        }
        Err(UnsandboxError::LdNotFound)
    }

    pub fn get_all_lib_paths(&self) -> Result<Vec<PathBuf>, UnsandboxError> {
        let out = Command::new("ldconfig").arg("-v").output()?;

        Ok(String::from_utf8(out.stdout)?
            .lines()
            .filter_map(|l| {
                if l.starts_with("\t") {
                    None
                } else {
                    Some(self.to_host_path(l.split(":").next().unwrap()))
                }
            })
            .collect::<Vec<_>>())
    }

    /// run a command unsandboxed. make sure to wrap paths in `FlatpakInfo::to_host_path()`
    pub fn run_unsandboxed(
        &self,
        command: Vec<CmdArg>,
        envs: Option<Vec<(String, CmdArg)>>,
        cwd: Option<PathBuf>,
        options: UnsandboxOptions,
    ) -> Result<Command, UnsandboxError> {
        let command = command
            .iter()
            .map(|x| x.into_string(self.clone()))
            .collect::<Vec<_>>();
        let envs = envs.map(|e| {
            e.iter()
                .map(|x| (x.0.clone(), x.1.into_string(self.clone())))
                .collect::<Vec<_>>()
        });
        log::debug!("Received command: {:?}", command);
        log::debug!("Received envs: {:?}", envs);
        log::debug!("Received cwd: {:?}", cwd);
        let lib_paths = self
            .get_all_lib_paths()?
            .iter()
            .map(|x| x.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(":");
        log::debug!("lib_paths: {:?}", lib_paths);
        let ld_path = self.get_ld_path()?;
        log::debug!("ld_path: {:?}", ld_path);
        let mut cmd = Command::new("flatpak-spawn");
        if options.clear_env {
            cmd.env_clear();
        }
        if options.attempt_env_translation {
            let other_envs = env::vars()
                .map(|(e, val)| (e, CmdArg::new_guess(val).into_string(self.clone())))
                .collect::<Vec<_>>();
            log::debug!(
                "Attempting env translation: {:?} to {:?}",
                other_envs,
                env::vars().collect::<Vec<_>>()
            );
            cmd.envs(other_envs);
        }
        cmd.arg("--host");
        cmd.arg(ld_path).arg("--library-path").arg(&lib_paths);
        cmd.args(command);
        if envs.is_some() {
            cmd.envs(envs.unwrap());
        }
        cmd.env("LD_LIBRARY_PATH", &lib_paths);
        if let Some(wd) = cwd {
            cmd.current_dir(CmdArg::new_path(wd).into_string(self.clone()));
        }
        log::debug!(
            "{:?} {:?} {:?} {:?}",
            cmd.get_program(),
            cmd.get_args(),
            cmd.get_envs(),
            cmd.get_current_dir()
        );
        Ok(cmd)
    }
}

#[derive(Clone, Debug)]
pub struct UnsandboxOptions {
    pub attempt_env_translation: bool,
    pub clear_env: bool
}

pub fn is_flatpaked() -> bool {
    Path::new("/.flatpak-info").exists()
}

pub fn has_flatpak_spawn_permission() -> Result<bool, UnsandboxError> {
    let connection = Connection::session()?;
    let pr = PeerProxy::new(
        &connection,
        "org.freedesktop.Flatpak",
        "/org/freedesktop/Flatpak/Development",
    )?;
    if let Err(_) = pr.ping() {
        Ok(false)
    } else {
        Ok(true)
    }
}
