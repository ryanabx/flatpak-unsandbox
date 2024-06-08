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
    /// use the flatpak's bundled libs
    #[arg(long)]
    use_bundled_libs: bool,
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
                .run_unsandboxed(cmd, envs, None, cli.use_bundled_libs)?
                .status()
            {
                Ok(out) => {
                    log::info!("Exit code: {:?}", out.code());
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
