use std::collections::HashMap;

use flatpak_unsandbox::UnsandboxError;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
/// flatpak-unsandbox: Run programs outside the flatpak sandbox
struct Cli {
    /// command to run (use )
    command: Vec<String>,
    /// environment variables to add
    #[arg(short, long)]
    env: Vec<String>,
    /// translate environment variables
    #[arg(long)]
    translate_env: bool,
    /// clears environment variables
    #[arg(long)]
    clear_env: bool,
}

fn main() -> Result<(), UnsandboxError> {
    simple_logger::init_with_level(log::Level::Trace).unwrap();
    if !flatpak_unsandbox::is_flatpaked() {
        log::error!("Run this command inside a flatpak!");
        return Err(UnsandboxError::NotSandboxed);
    } else {
        let cli = Cli::parse();
        if !cli.command.is_empty() {
            log::debug!("Command: {:?} :: Envs: {:?}", cli.command, cli.env);
            let cmd = cli
                .command
                .iter()
                .map(|x| flatpak_unsandbox::CmdArg::new_guess(x.clone()))
                .collect::<Vec<_>>();
            let envs = cli
                .env
                .iter()
                .map(|x| {
                    let (x1, x2) = x.split_once("=").unwrap();
                    (
                        x1.to_string(),
                        flatpak_unsandbox::CmdArg::new_guess(x2.to_string()),
                    )
                })
                .collect::<HashMap<_, _>>();
            let info = flatpak_unsandbox::FlatpakInfo::new()?;

            match info
                .run_unsandboxed(
                    cmd,
                    envs,
                    None,
                    flatpak_unsandbox::UnsandboxOptions {
                        translate_env: cli.translate_env,
                        clear_env: cli.clear_env,
                    },
                )?
                .output()
            {
                Ok(out) => {
                    log::info!("stdout: {}", String::from_utf8(out.stdout).unwrap());
                    log::info!("stderr: {}", String::from_utf8(out.stderr).unwrap());
                    log::info!("Exit code: {:?}", out.status);
                    Ok(())
                }
                Err(e) => {
                    log::error!("Command ran into an issue: {:?}", e);
                    Err(e.into())
                }
            }
        } else {
            Ok(())
        }
    }
}
