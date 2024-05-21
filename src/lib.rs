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

#[derive(Clone, Debug)]
pub struct Program {
    pub path: PathBuf,
    pub args: Vec<String>,
}

impl Program {
    pub fn new(file: impl Into<PathBuf>, args: Option<Vec<String>>) -> Self {
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
            args: env::args().skip(1).collect::<Vec<_>>(),
        }
    }
}

/// Runs this program, or an optional `program` outside of the flatpak sandbox.
/// > **NOTE:** You must have the permission `--talk-name=org.freedesktop.Flatpak` enabled
/// Returns `true` if the program was executed by this function, `false` otherwise.
pub fn unsandbox(program: Option<Program>) -> Result<bool, UnsandboxError> {
    let program = program.unwrap_or_default();
    let base_dir = if is_flatpaked() {
        get_flatpak_base_dir()?
    } else {
        return Ok(false);
    };
    let app_dir = &program.path;
    let program_dir = base_dir.join(if app_dir.is_absolute() {
        app_dir.strip_prefix("/app").unwrap()
    } else {
        app_dir.strip_prefix("app").unwrap()
    });
    log::debug!("Got program: {:?}", program);
    log::debug!("Effective base directory on host: {:?}", program_dir);
    let args = program
        .args
        .iter()
        .map(|x| {
            let y = Path::new(x);
            if y.try_exists().is_ok_and(|v| v) {
                base_dir.join(y).to_string_lossy().to_string()
            } else {
                x.into()
            }
        })
        .collect::<Vec<_>>();
    // Run program. This will halt execution on the main thread.
    let _ = Command::new("flatpak-spawn")
        .arg("--host")
        .arg(program_dir)
        .args(args)
        .status()?;
    Ok(true)
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
