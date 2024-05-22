use std::{
    env, io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    process::Command,
};

use gio::prelude::FileExt;
use glib::{KeyFile, KeyFileFlags};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UnsandboxError {
    #[error("Program failed `{0}`")]
    ExecutionError(#[from] io::Error),
    #[error("Glib had a problem `{0}`")]
    GlibError(#[from] glib::Error),
}

#[derive(Debug, Clone)]
pub enum ProgramArg {
    Value(String),
    Path { path: PathBuf, in_sandbox: bool },
}

#[derive(Clone, Debug)]
pub struct Program {
    pub path: PathBuf,
    pub args: Vec<ProgramArg>,
    pub envs: Vec<(String, String)>,
}

impl From<ProgramArg> for String {
    fn from(value: ProgramArg) -> Self {
        match value {
            ProgramArg::Value(val) => val,
            ProgramArg::Path { path, in_sandbox } => {
                if in_sandbox && is_flatpaked() {
                    path_as_unsandboxed(&path)
                        .unwrap()
                        .to_string_lossy()
                        .to_string()
                } else {
                    path.to_string_lossy().to_string()
                }
            }
        }
    }
}

impl From<String> for ProgramArg {
    fn from(value: String) -> Self {
        Self::Value(value)
    }
}

impl Program {
    pub fn new(
        file: impl Into<PathBuf>,
        args: Option<Vec<ProgramArg>>,
        envs: Option<Vec<(String, String)>>,
    ) -> Self {
        Program {
            path: file.into(),
            args: args.unwrap_or_default(),
            envs: envs.unwrap_or_default(),
        }
    }
}

impl Default for Program {
    fn default() -> Self {
        Self {
            path: env::current_exe().unwrap(),
            args: env::args()
                .skip(1)
                .map(|x| x.into())
                .collect::<Vec<ProgramArg>>(),
            envs: env::vars().collect::<Vec<_>>(),
        }
    }
}

/// Runs this program, or an optional `program` outside of the flatpak sandbox.
/// If no other program is specified, and the app is outside the sandbox, returns `false`
/// > **NOTE:** You must have the permission `--talk-name=org.freedesktop.Flatpak` enabled
/// Returns `true` if the program was executed by this function, `false` otherwise.
pub fn unsandbox(program: Option<Program>) -> Result<Option<Command>, UnsandboxError> {
    if !is_flatpaked() && program.is_none() {
        return Ok(None);
    }
    let program = program.unwrap_or_default();
    let program_dir = if is_flatpaked() {
        path_as_unsandboxed(&program.path)?
    } else {
        program.path.to_path_buf()
    };
    log::debug!("Got program: {:?}", program);
    let args = program
        .args
        .iter()
        .map(|x| String::from(x.clone()))
        .collect::<Vec<_>>();
    let envs = program.envs;
    // Run program. This will halt execution on the main thread.
    log::info!(
        "Command: '{}'",
        if is_flatpaked() {
            format!("flatpak-spawn --host {:?} {:?}", program_dir, args)
        } else {
            format!("{:?} {:?}", program_dir, args)
        }
    );
    let cmd = if is_flatpaked() {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg(program_dir).args(args).envs(envs);
        c
    } else {
        let mut c = Command::new(program_dir);
        c.args(args).envs(envs);
        c
    };
    Ok(Some(cmd))
}

fn path_as_unsandboxed(path: &Path) -> Result<PathBuf, glib::Error> {
    let flatpak_info = KeyFile::new();
    let data = gio::File::for_path("/.flatpak-info");
    flatpak_info.load_from_bytes(
        &data.load_bytes(gio::Cancellable::current().as_ref())?.0,
        KeyFileFlags::empty(),
    )?;
    log::debug!(
        "Path of instance: {:?}",
        flatpak_info.string("Instance", "app-path")?
    );
    Ok(
        Path::new(&flatpak_info.string("Instance", "app-path")?.to_string()).join(
            if path.is_absolute() {
                path.strip_prefix("/app").unwrap()
            } else {
                path.strip_prefix("app").unwrap()
            },
        ),
    )
}

fn get_flatpak_base_dir() -> Result<PathBuf, glib::Error> {
    let flatpak_info = KeyFile::new();
    let data = gio::File::for_path("/.flatpak-info");
    flatpak_info.load_from_bytes(
        &data.load_bytes(gio::Cancellable::current().as_ref())?.0,
        KeyFileFlags::empty(),
    )?;
    log::debug!(
        "Path of instance: {:?}",
        flatpak_info.string("Instance", "app-path")?
    );
    Ok(Path::new(&flatpak_info.string("Instance", "app-path")?.to_string()).to_owned())
}

fn is_flatpaked() -> bool {
    Path::new("/.flatpak-info").exists()
}
