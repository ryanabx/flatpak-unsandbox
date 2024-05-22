use std::{
    env, io,
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
    pub fn new(file: impl Into<PathBuf>, args: Option<Vec<ProgramArg>>) -> Self {
        Program {
            path: file.into(),
            args: args.unwrap_or_default(),
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
        }
    }
}

/// Runs this program, or an optional `program` outside of the flatpak sandbox.
/// > **NOTE:** You must have the permission `--talk-name=org.freedesktop.Flatpak` enabled
/// Returns `true` if the program was executed by this function, `false` otherwise.
pub fn unsandbox(program: Option<Program>) -> Result<bool, UnsandboxError> {
    if !is_flatpaked() && program.is_none() {
        return Ok(false);
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
    // Run program. This will halt execution on the main thread.
    let _ = Command::new("flatpak-spawn")
        .arg("--host")
        .arg(program_dir)
        .args(args)
        .status()?;
    Ok(true)
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
