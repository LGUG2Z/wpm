# wpm

Simple user process management for Windows.

<p>
  <a href="https://techforpalestine.org/learn-more">
    <img alt="Tech for Palestine" src="https://badge.techforpalestine.org/default">
  </a>
  <img alt="GitHub Workflow Status" src="https://img.shields.io/github/actions/workflow/status/LGUG2Z/wpm/.github/workflows/windows.yaml">
  <img alt="GitHub all releases" src="https://img.shields.io/github/downloads/LGUG2Z/wpm/total">
  <img alt="GitHub commits since latest release (by date) for a branch" src="https://img.shields.io/github/commits-since/LGUG2Z/wpm/latest">
  <a href="https://discord.gg/mGkn66PHkx">
    <img alt="Discord" src="https://img.shields.io/discord/898554690126630914">
  </a>
  <a href="https://github.com/sponsors/LGUG2Z">
    <img alt="GitHub Sponsors" src="https://img.shields.io/github/sponsors/LGUG2Z">
  </a>
  <a href="https://ko-fi.com/lgug2z">
    <img alt="Ko-fi" src="https://img.shields.io/badge/kofi-tip-green">
  </a>
  <a href="https://notado.app/feeds/jado/software-development">
    <img alt="Notado Feed" src="https://img.shields.io/badge/Notado-Subscribe-informational">
  </a>
  <a href="https://www.youtube.com/channel/UCeai3-do-9O4MNy9_xjO6mg?sub_confirmation=1">
    <img alt="YouTube" src="https://img.shields.io/youtube/channel/subscribers/UCeai3-do-9O4MNy9_xjO6mg">
  </a>
</p>

_wpm_ is a simple user process manager for Microsoft Windows 11 and above.

_wpm_ allows you to start, stop and manage user level background processes as defined in unit files.

_wpm_ is a free and educational source project, and one that encourages you to make charitable donations if you find
the software to be useful and have the financial means.

I encourage you to make a charitable donation to
the [Palestine Children's Relief Fund](https://pcrf1.app.neoncrm.com/forms/gaza-recovery) or contributing to
a [Gaza Funds campaign](https://gazafunds.com) before you consider sponsoring me on GitHub.

[GitHub Sponsors is enabled for this project](https://github.com/sponsors/LGUG2Z). Unfortunately I don't have anything
specific to offer besides my gratitude and shout outs at the end of _komorebi_ live development videos and tutorials.

If you would like to tip or sponsor the project but are unable to use GitHub Sponsors, you may also sponsor
through [Ko-fi](https://ko-fi.com/lgug2z).

# Installation

While this project is in a pre-release state, you can install `wpmd` and `wpmctl` using `cargo`:

```shell
cargo install --git https://github.com/LGUG2Z/wpm wpmd
cargo install --git https://github.com/LGUG2Z/wpm wpmctl
```

# Usage

- Create unit files in `~/.config/wpm` - as an example, here is my `kanata.toml` unit file:

```toml
[unit]
name = "kanata"

[service]
executable = "kanata.exe"
arguments = ["-c", "$USERPROFILE/minimal.kbd", "--port", "9999"]
# next one isn't actually in my config, but here as an example ;)
environment = [["SOME_ENV_VAR", "bla-bla-bla"]]
```

- The full schema can be found [here](./wpmd/src/unit.rs) and is likely to change during this early development phase
- `$USERPROFILE` is a specially handled string in both `arguments` and `environment` which will be replaced with your home dir
- Run `wpmd` to start the daemon, this will load all unit files in `~/.config/wpm`
- Run `wpmctl start kanata` (or whatever your unit name is) to start the process
- Run `wpmctl log kanata` (or whatever your unit name is) to log the output of the process
- Run `wpmctl stop kanata` (or whatever your unit name is) to stop the process
- Run `wpmctl reload` to reload all unit definitions (useful if you're making changes)

# Contribution Guidelines

If you would like to contribute to `wpm` please take the time to carefully read the guidelines below.

## Commit hygiene

- Flatten all `use` statements
- Run `cargo +stable clippy` and ensure that all lints and suggestions have been addressed before committing
- Run `cargo +nightly fmt --all` to ensure consistent formatting before committing
- Use `git cz` with
  the [Commitizen CLI](https://github.com/commitizen/cz-cli#conventional-commit-messages-as-a-global-utility) to prepare
  commit messages
- Provide **at least** one short sentence or paragraph in your commit message body to describe your thought process for the
  changes being committed

## License

`wpm` is licensed under the [Komorebi 1.0.0 license](./LICENSE.md), which
is a fork of the [PolyForm Strict 1.0.0
license](https://polyformproject.org/licenses/strict/1.0.0). On a high level
this means that you are free to do whatever you want with `wpm` for
personal use other than redistribution, or distribution of new works (i.e.
hard-forks) based on the software.

Anyone is free to make their own fork of `wpm` with changes intended
either for personal use or for integration back upstream via pull requests.

The [Komorebi 1.0.0 License](./LICENSE.md) does not permit any kind of
commercial use.

### Contribution licensing

Contributions are accepted with the following understanding:

- Contributed content is licensed under the terms of the 0-BSD license
- Contributors accept the terms of the project license at the time of contribution

By making a contribution, you accept both the current project license terms, and that all contributions that you have
made are provided under the terms of the 0-BSD license.

#### Zero-Clause BSD

```
Permission to use, copy, modify, and/or distribute this software for
any purpose with or without fee is hereby granted.

THE SOFTWARE IS PROVIDED “AS IS” AND THE AUTHOR DISCLAIMS ALL
WARRANTIES WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES
OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE
FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY
DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN
AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT
OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
```
