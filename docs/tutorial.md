# Tutorial

For the tutorial, we will use `wpm` to set up a keyboard-focused desktop
environment on a brand new virtual machine which uses `kanata` to enable
QMK-style keyboard layers, `whkd` to enable programmable hotkeys, `komorebi`
to enable tiling window management, and `komorebi-bar` as a status bar.

One you have completed the tutorial, you should have a good idea of how `wpm`
can be used to model and enforce constraints in use cases from customized
desktops to complex local development environments and more.

## Create a new Virtual Machine

* Open Hyper-V Manager
* Select "Quick Create"
* Select "Windows 11 dev environment"
* Select "Create Virtual Machine"

## Install scoop

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
Invoke-RestMethod -Uri https://get.scoop.sh | Invoke-Expression
```

## Install wpm

Install `scoop` and then install `wpm`

```powershell
scoop install git # need this to be able to add the extras bucket

scoop bucket add extras
scoop install wpm
```

## Generate example units

Generate some example unit files in `~/.config/wpm`

```powershell
wpmctl examplegen $(wpm units)
```

You can `ls` the directory to make sure they have been generated

```powershell
PS C:\Users\User> ls $(wpmctl units)


    Directory: C:\Users\User\.config\wpm


Mode                 LastWriteTime         Length Name
----                 -------------         ------ ----
-a----          4/6/2025   3:02 PM            503 desktop.json
-a----          4/6/2025   3:02 PM            883 kanata.json
-a----          4/6/2025   3:02 PM           1032 komokana.json
-a----          4/6/2025   3:02 PM           1026 komorebi-bar.json
-a----          4/6/2025   3:02 PM           1417 komorebi.json
-a----          4/6/2025   3:02 PM            971 mousemaster.json
-a----          4/6/2025   3:02 PM            835 whkd.json
```

## Start `wpmd`

Run `wpmd` in terminal to start the process manager - this will automatically
download all required packages and configuration files before starting to
listen for commands

```text
PS C:\Users\User> wpmd
2025-04-06T22:07:42.124299Z  INFO wpm::process_manager: desktop: registered unit
2025-04-06T22:07:42.126354Z  INFO wpm::unit: kanata: adding resource C:\Users\User\AppData\Local\wpm\store\gist.githubusercontent.com_LGUG2Z_bbafc51ddde2bd1462151cfcc3f7f489_raw_28e24c4a493166fa866ae24ebc4ed8df7f164bd1\minimal.clj to store
2025-04-06T22:07:42.207182Z  INFO wpm::unit: installing scoop manifest https://raw.githubusercontent.com/ScoopInstaller/Extras/8a6d8ff0f3963611ae61fd9f45ff36e3c321c8b5/bucket/kanata.json
Installing 'kanata' (1.8.1) [64bit] from 'https://raw.githubusercontent.com/ScoopInstaller/Extras/8a6d8ff0f3963611ae61fd9f45ff36e3c321c8b5/bucket/kanata.json'
Loading kanata.exe from cache
Checking hash of kanata.exe ... ok.
Linking ~\scoop\apps\kanata\current => ~\scoop\apps\kanata\1.8.1
Creating shim for 'kanata'.
'kanata' (1.8.1) was installed successfully!
Notes
-----
Configuration Guide: https://github.com/jtroo/kanata/blob/main/docs/config.adoc

2025-04-06T22:07:44.513651Z  INFO wpm::process_manager: kanata: registered unit
2025-04-06T22:07:44.515218Z  INFO wpm::unit: komokana: adding resource C:\Users\User\AppData\Local\wpm\store\raw.githubusercontent.com_LGUG2Z_komokana_refs_tags_v0.1.5\komokana.example.yaml to store
2025-04-06T22:07:44.592705Z  INFO wpm::unit: installing scoop manifest https://raw.githubusercontent.com/ScoopInstaller/Extras/e633292b4e1101273caac59ffcb4a7ce7ee7a2e8/bucket/komokana.json
Installing 'komokana' (0.1.5) [64bit] from 'https://raw.githubusercontent.com/ScoopInstaller/Extras/e633292b4e1101273caac59ffcb4a7ce7ee7a2e8/bucket/komokana.json'
Loading komokana-0.1.5-x86_64-pc-windows-msvc.zip from cache
Checking hash of komokana-0.1.5-x86_64-pc-windows-msvc.zip ... ok.
Extracting komokana-0.1.5-x86_64-pc-windows-msvc.zip ... done.
Linking ~\scoop\apps\komokana\current => ~\scoop\apps\komokana\0.1.5
Creating shim for 'komokana'.
'komokana' (0.1.5) was installed successfully!
'komokana' suggests installing 'extras/komorebi'.

2025-04-06T22:07:46.792651Z  INFO wpm::process_manager: komokana: registered unit
2025-04-06T22:07:46.793421Z  INFO wpm::unit: komorebi-bar: adding resource C:\Users\User\AppData\Local\wpm\store\raw.githubusercontent.com_LGUG2Z_komorebi_refs_tags_v0.1.35_docs\komorebi.bar.example.json to store
2025-04-06T22:07:46.820347Z  INFO wpm::unit: installing scoop manifest https://raw.githubusercontent.com/ScoopInstaller/Extras/8e21dc2cd902b865d153e64a078d97d3cd0593f7/bucket/komorebi.json
Installing 'komorebi' (0.1.35) [64bit] from 'https://raw.githubusercontent.com/ScoopInstaller/Extras/8e21dc2cd902b865d153e64a078d97d3cd0593f7/bucket/komorebi.json'
Loading komorebi-0.1.35-x86_64-pc-windows-msvc.zip from cache
Checking hash of komorebi-0.1.35-x86_64-pc-windows-msvc.zip ... ok.
Extracting komorebi-0.1.35-x86_64-pc-windows-msvc.zip ... done.
Linking ~\scoop\apps\komorebi\current => ~\scoop\apps\komorebi\0.1.35
Creating shim for 'komorebi'.
Creating shim for 'komorebic'.
Creating shim for 'komorebic-no-console'.
Making C:\Users\User\scoop\shims\komorebic-no-console.exe a GUI binary.
Creating shim for 'komorebi-gui'.
Creating shim for 'komorebi-bar'.
'komorebi' (0.1.35) was installed successfully!
Notes
-----
Check out the quickstart guide on https://lgug2z.github.io/komorebi
'komorebi' suggests installing 'extras/autohotkey'.
'komorebi' suggests installing 'extras/whkd'.

2025-04-06T22:07:49.420615Z  INFO wpm::process_manager: komorebi-bar: registered unit
2025-04-06T22:07:49.421177Z  INFO wpm::unit: komorebi: adding resource C:\Users\User\AppData\Local\wpm\store\raw.githubusercontent.com_LGUG2Z_komorebi_refs_tags_v0.1.35_docs\komorebi.example.json to store
2025-04-06T22:07:49.442994Z  INFO wpm::process_manager: komorebi: registered unit
2025-04-06T22:07:49.443741Z  INFO wpm::unit: whkd: adding resource C:\Users\User\AppData\Local\wpm\store\raw.githubusercontent.com_LGUG2Z_komorebi_refs_tags_v0.1.35_docs\whkdrc.sample to store
2025-04-06T22:07:49.470394Z  INFO wpm::unit: installing scoop manifest https://raw.githubusercontent.com/ScoopInstaller/Extras/112fd691392878f8c4e9e9703dde3d1d182941e3/bucket/whkd.json
Installing 'whkd' (0.2.7) [64bit] from 'https://raw.githubusercontent.com/ScoopInstaller/Extras/112fd691392878f8c4e9e9703dde3d1d182941e3/bucket/whkd.json'
Loading whkd-0.2.7-x86_64-pc-windows-msvc.zip from cache
Checking hash of whkd-0.2.7-x86_64-pc-windows-msvc.zip ... ok.
Extracting whkd-0.2.7-x86_64-pc-windows-msvc.zip ... done.
Linking ~\scoop\apps\whkd\current => ~\scoop\apps\whkd\0.2.7
Creating shim for 'whkd'.
'whkd' (0.2.7) was installed successfully!

2025-04-06T22:07:51.580342Z  INFO wpm::process_manager: whkd: registered unit
2025-04-06T22:07:51.580712Z  INFO wpmd: listening on wpmd.sock
```

## Start the units

The dependency graph of our example units looks like this

* `komorebi-bar` depends on `komorebi`
* `komorebi` depends on `whkd` and `kanata`

So we can run `wpmctl start komorebi-bar` to ensure that `whkd`, `kanata`,
`komorebi` and `komorebi-bar` are all started and passing their healthchecks.


```text
2025-04-06T22:12:26.419780Z  INFO wpmd: received socket message: Start(["komorebi-bar"])
2025-04-06T22:12:26.420163Z  INFO wpmd: successfully queued socket message
2025-04-06T22:12:26.420204Z  INFO wpm::process_manager: komorebi-bar: requires komorebi
2025-04-06T22:12:26.420689Z  INFO wpm::process_manager: komorebi: requires whkd
2025-04-06T22:12:26.421003Z  INFO wpm::unit: whkd: starting unit
2025-04-06T22:12:26.424052Z  INFO wpm::unit: whkd: running pid 11716 liveness healthcheck (1s)
2025-04-06T22:12:27.441812Z  INFO wpm::unit: whkd: passed healthcheck
2025-04-06T22:12:27.442303Z  INFO wpm::process_manager: komorebi: requires kanata
2025-04-06T22:12:27.442572Z  INFO wpm::unit: kanata: starting unit
2025-04-06T22:12:27.446427Z  INFO wpm::unit: kanata: running pid 9520 liveness healthcheck (1s)
2025-04-06T22:12:28.466568Z  INFO wpm::unit: kanata: passed healthcheck
2025-04-06T22:12:28.466976Z  INFO wpm::unit: komorebi: starting unit
2025-04-06T22:12:28.471147Z  INFO wpm::unit: komorebi: running command healthcheck - C:\Users\User\scoop\shims\komorebic.exe state (1s)
2025-04-06T22:12:29.503620Z  INFO wpm::unit: komorebi: passed healthcheck
2025-04-06T22:12:29.503983Z  INFO wpm::unit: komorebi-bar: starting unit
2025-04-06T22:12:29.507663Z  INFO wpm::unit: komorebi-bar: running pid 9860 liveness healthcheck (1s)
2025-04-06T22:12:30.529935Z  INFO wpm::unit: komorebi-bar: passed healthcheck
```

## Shutdown

You can press `ctrl-c` on the terminal window running `wpmd` to trigger a shutdown,
which will ensure that all processes started in the previous steps are shutdown
cleanly with their shutdown hooks respected.

```text
2025-04-06T22:20:02.303289Z  INFO wpm::process_manager: wpmd: shutting down process manager
2025-04-06T22:20:02.303496Z  INFO wpm::process_manager: whkd: stopping unit
2025-04-06T22:20:02.303622Z  INFO wpm::process_manager: whkd: sending kill signal to 70000
2025-04-06T22:20:02.306467Z  INFO wpm::process_manager: whkd: process 70000 successfully terminated
2025-04-06T22:20:02.306587Z  INFO wpm::process_manager: komorebi-bar: stopping unit
2025-04-06T22:20:02.306714Z  INFO wpm::process_manager: komorebi-bar: sending kill signal to 40192
2025-04-06T22:20:02.352968Z  INFO wpm::process_manager: komorebi-bar: process 40192 successfully terminated
2025-04-06T22:20:02.356650Z  INFO wpm::process_manager: komorebi: stopping unit
2025-04-06T22:20:02.356777Z  INFO wpm::process_manager: komorebi: executing shutdown command - C:\Users\User\scoop\shims\komorebic.exe stop
2025-04-06T22:20:02.453174Z  INFO wpm::unit: komorebi: executing cleanup command - C:\Users\User\scoop\shims\komorebic.exe restore-windows
2025-04-06T22:20:02.792875Z  INFO wpm::process_manager: komorebi: sending kill signal to 17448
2025-04-06T22:20:02.793018Z  INFO wpm::process_manager: komorebi: process 17448 successfully terminated
2025-04-06T22:20:02.793137Z  INFO wpm::process_manager: komorebi: executing cleanup command - C:\Users\User\scoop\shims\komorebic.exe restore-windows
2025-04-06T22:20:02.811672Z  INFO wpm::process_manager: kanata: stopping unit
2025-04-06T22:20:02.811799Z  INFO wpm::process_manager: kanata: sending kill signal to 70136
2025-04-06T22:20:02.814617Z  INFO wpm::process_manager: kanata: process 70136 successfully terminated
```
