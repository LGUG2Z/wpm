# Getting started

`wpm` is a tiling window manager for Windows that is comprised of two main
binaries, `wpmd.exe`, which contains the process management daemon itself, and
`wpmctl.exe`, which is the main way to send commands to the process management
daemon.

## Installation

`wpm` is available pre-built to install via
[Scoop](https://scoop.sh/#/apps?q=wpm) and
[WinGet](https://winget.run/pkg/LGUG2Z/wpm), and you may also build
it from [source](https://github.com/LGUG2Z/wpm) if you would prefer.

- [Scoop](#scoop)
- [WinGet](#winget)
- [Building from source](#building-from-source)
- [Offline](#offline)

## Long path support

It is highly recommended that you enable support for long paths in Windows by
running the following command in an Administrator Terminal before installing
`wpm`.

```powershell
Set-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem' -Name 'LongPathsEnabled' -Value 1
```

## Scoop

Make sure you have installed [`scoop`](https://scoop.sh) and verified that
installed binaries are available in your `$PATH` before proceeding.

Issues with `wpm` and related commands not being recognized in the
terminal ultimately come down to the `$PATH` environment variable not being
correctly configured by your package manager and **should not** be raised as
bugs or issues either on the `wpm` GitHub repository or Discord server.

### Install wpm

First add the extras bucket

```powershell
scoop bucket add extras
```

Then install the `wpm` package using `scoop install`

```powershell
scoop install wpm
```

## WinGet

Make sure you have installed the latest version of
[`winget`](https://learn.microsoft.com/en-us/windows/package-manager/winget/)
and verified that installed binaries are available in your `$PATH` before
proceeding.

Issues with `wpmd` and related commands not being recognized in the
terminal ultimately come down to the `$PATH` environment variable not being
correctly configured by your package manager and **should not** be raised as
bugs or issues either on the `wpm` GitHub repository or Discord server.

### Install wpm

Install the `wpm` packages using `winget install`

```powershell
winget install LGUG2Z.wpm
```

## Building from source

Make sure you have installed [`rustup`](https://rustup.rs), a stable `rust`
compiler toolchain, and the Visual Studio [Visual Studio
prerequisites](https://rust-lang.github.io/rustup/installation/windows-msvc.html).

Clone the git repository, enter the directory, and build the following binaries:

```powershell
cargo +stable install --path wpm --locked
cargo +stable install --path wpmd --locked
```

If the binaries have been built and added to your `$PATH` correctly, you should
see some output when running `wpmd --help` and `wpmctl --help`

### Offline

Download the latest [wpm](https://github.com/LGUG2Z/wpm/releases)
MSI installer on an internet-connected computer, then copy it to
an offline machine to install.

## Upgrades

Before upgrading, make sure that `wpmd` is stopped. This is to ensure that all
the current `wpm`-related exe files can be replaced without issue.

Then, depending on whether you installed via `scoop` or `winget`, you can run
the appropriate command:

```powershell
# for winget
winget upgrade LGUG2Z.wpm
```

```powershell
# for scoop
scoop update wpm
```

## Uninstallation

Before uninstalling, first ensure that `wpmd` is stopped.

Then, depending on whether you installed with Scoop or WinGet, run `scoop
uninstall wpm` or `winget uninstall LGUG2Z.wpm`.

Finally, you can run the following commands in a PowerShell prompt to clean up
files created by the `quickstart` command and any other runtime files:

```powershell
rm -r -Force $Env:USERPROFILE\.config\wpm
rm -r -Force $Env:LOCALAPPDATA\wpm
```
