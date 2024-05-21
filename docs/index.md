# flatpak-unsandbox

flatpak-unsandbox is a simple crate that lets you run an application from within the flatpak sandbox outside the flatpak sandbox!

## Simple usage


Run your program unflatpaked

```rust
// src/main.rs
fn main() -> Result<(), MyError> {
    if flatpak_unsandbox::unsandbox(None)? {
        return Ok(())
    }
    // Unsandboxed functionality here...
}
```

Run another program unflatpaked

```rust
// src/main.rs
fn main() -> Result<(), MyError> {
    // Sandboxed functionality
    // Ensure this other program ran
    if !flatpak_unsandbox::unsandbox(Some(Program::new(
        "/libexec/my-agent-program", None)
        ))? {
        return Ok(())
    }
    // More sandboxed functionality here...
}
```