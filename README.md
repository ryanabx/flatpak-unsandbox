# Flatpak Unsandbox

This rust crate allows rust flatpak apps to run themselves outside of the sandbox. The unsandboxed app will link to the flatpak's libraries, so dependencies won't be an issue.

## Example usage

```rust
fn main() -> Result<(), UnsandboxError> {
    // Will error if not flatpacked or no permissions
    let my_info = FlatpakInfo::new()?;
    let cmd = vec![
        CmdArg::new_path("/app/bin/flatrun-agent"),
        CmdArg::new_string("--bundle".into()),
        CmdArg::new_path("/var/home/1000/inkscape.flatpak"),
    ];
    let env = vec![(
        "PATH_TO_FLATPAK".to_string(),
        CmdArg::new_path("/app/bin/flatpak"),
    )];
    let out = my_info
        .run_unsandboxed(cmd, Some(env), None)?
        .output()
        .unwrap();
    println!("{}", String::from_utf8(out.stdout).unwrap());
    Ok(())
}
```

## Contributing

If there are issues with the crate, you may submit a bug report or attempt to fix the issue and make a PR!