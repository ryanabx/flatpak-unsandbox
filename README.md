# Flatpak Unsandbox

This rust crate allows rust flatpak apps to run themselves outside of the sandbox.

## What this crate is not for:

* Getting around restrictions in Flatpak - Flatpak's sandbox is immensely useful to ensure user security, and this crate is not for maliciously skirting around restrictions. 
> **NOTE:** you must have the `--talk-name=org.freedesktop.Flatpak` permission enabled and already that is the biggest hole in the sandbox we can make. Use this library with extreme caution.

* Running any regular app that you'd rather not figure out the sandboxing for - Please please **please** use Flatpak's sandboxing whenever possible.

## What this crate is for:

* Apps that **must** run on the host, and have no other choice.

* Apps that need to run a specific part of its functionality on the host

## Examples of apps that would need this crate:

* Apps that modify and use the host's flatpak installations: (for example, [Flatrun](https://github.com/ryanabx/flatrun))

* Apps that aren't built by the packager, and have otherwise no way to package the app under Flatpak's sandboxing (very rare).

* Apps that require system services

> **WARNING:** Packaging these apps through flatpak might be a bad idea because we can't ensure dependencies exist on the host system!

## Contributing

Not much needs to be updated for this crate, but if there are issues with it, you may submit a bug report or attempt to fix the issue and make a PR!