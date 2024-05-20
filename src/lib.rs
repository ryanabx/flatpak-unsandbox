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

/// Runs this program, or an optional `program` outside of the flatpak sandbox.
/// > **NOTE:** You must have the permission `--talk-name=org.freedesktop.Flatpak` enabled
pub fn unsandbox(program: Option<PathBuf>) -> Result<(), UnsandboxError> {
    let program = program.unwrap_or(env::current_exe()?);
    let program = if is_flatpaked() {
        get_flatpak_app_dir(&program)?
    } else {
        program
    };
    let args = env::args();
    // Run program. This will halt execution on the main thread.
    let _ = Command::new("flatpak")
        .arg("spawn")
        .arg(program)
        .args(args)
        .status()?;
    Ok(())
}

fn get_flatpak_app_dir(app_dir: &Path) -> Result<PathBuf, glib::Error> {
    let flatpak_info = KeyFile::new();
    let data = gio::File::for_path("/.flatpak-info");
    flatpak_info.load_from_bytes(
        &data.load_bytes(gio::Cancellable::current().as_ref())?.0,
        KeyFileFlags::empty(),
    )?;
    Ok(Path::new(
        &flatpak_info
            .string("Instance", "app-path")
            .unwrap()
            .to_string(),
    )
    .join(app_dir))
}

fn is_flatpaked() -> bool {
    Path::new("/.flatpak-info").exists()
}
