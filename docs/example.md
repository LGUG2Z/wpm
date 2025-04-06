# Units

Unit files define everything that `wpm` needs to be able to manage a process.
Below is a non-trivial unit file for
[komorebi](https://github.com/LGUG2Z/komorebi) which has been annotated to help
explain the various configuration options available.

1. [`Requires`](https://wpm.lgug2z.com/schema#Unit_Requires) is used to specify
   dependency relationships between different processes. In this example,
   `komorebi` depends on `whkd` and `kanata`, which means that if you attempt
   to `wpmctl start komorebi`, `wpm` will check if `whkd` and `kanata` are both
   healthy and running before it attempts to start `komorebi`.
1. [`Resources`](https://wpm.lgug2z.com/schema#Resources) is used to provide
   URLs to additional resources that the unit requires in order to run, such as
   configuration files. The key given to each URL here can be used to reference
   the cached location of the downloaded file on disk, for example, when
   passing a configuration file as an argument or an environment variable.
1. [`Kind`](https://wpm.lgug2z.com/schema#Service_Kind) is used to tell `wpm`
   if this process continues running when launched (`Simple`), runs and then
   exits (`OneShot`), or runs and exits after forking a new process
   (`Forking`).
1. Every unit has a complete set of lifecycle hooks available
   ([`ExecStartPre`](https://wpm.lgug2z.com/schema#Service_ExecStartPre),
   [`ExecStartPost`](https://wpm.lgug2z.com/schema#Service_ExecStartPost),
   [`ExecStop`](https://wpm.lgug2z.com/schema#Service_ExecStop),
   [`ExecStopPost`](https://wpm.lgug2z.com/schema#Service_ExecStopPost)) to
   specify any preflight or cleanup tasks a process might require.
1. [`Executable`](https://wpm.lgug2z.com/schema#Service_ExecStart_Executable)
   can reference either an binary in the system `$PATH`, a remote URL and
   checksum hash, or a [Scoop](https://scoop.sh) package manifest. Remote
   binaries will be cached in a local store for future use, as will Scoop
   packages. The latter two approaches can be used to pin binary dependencies
   to exact versions (ie. enforcing service dependency consistency across a
   team)
1. Keys declared in [`Resources`](https://wpm.lgug2z.com/schema#Resources) can
   be referenced as arguments using the `Resources.KEY` syntax inside of double
   curly braces.
1. `$USERPROFILE` will resolve to `C:\Users\<YourUser>` when used in
   `Arguments` and `Environment`
1. [`Healthcheck`](https://wpm.lgug2z.com/schema#Service_Healthcheck) is used
   to tell `wpm` how to validate the health of a process. This can be done by
   invoking a command until it returns with a successful exit code, or by
   checking the liveness of a process after a fixed period of time.

{% raw %}

```json
{
  "$schema": "https://raw.githubusercontent.com/LGUG2Z/wpm/refs/heads/master/schema.unit.json",
  "Unit": {
    "Name": "komorebi",
    "Description": "Tiling window management for Windows",
    // [1]
    "Requires": ["whkd", "kanata"]
  },
  // [2]
  "Resources": {
    "CONFIGURATION_FILE": "https://raw.githubusercontent.com/LGUG2Z/komorebi/refs/tags/v0.1.35/docs/komorebi.example.json"
  },
  "Service": {
    // [3]
    "Kind": "Simple",
    // [4]
    "ExecStartPre": [
      {
        "Executable": "komorebic.exe",
        "Arguments": ["fetch-asc"]
      }
    ],
    "ExecStart": {
      // [5]
      "Executable": {
        "Package": "komorebi",
        "Version": "0.1.35",
        "Manifest": "https://raw.githubusercontent.com/ScoopInstaller/Extras/8e21dc2cd902b865d153e64a078d97d3cd0593f7/bucket/komorebi.json",
        "Target": "komorebi.exe"
      },
      "Arguments": [
        "--config",
        // [6]
        "{{ Resources.CONFIGURATION_FILE }}"
      ],
      "Environment": [
        [
          "KOMOREBI_CONFIG_HOME",
          // [7]
          "$USERPROFILE/.config/komorebi"
        ]
      ]
    },
    // [4]
    "ExecStop": [
      {
        "Executable": "komorebic.exe",
        "Arguments": ["stop"]
      }
    ],
    // [4]
    "ExecStopPost": [
      {
        "Executable": "komorebic.exe",
        "Arguments": ["restore-windows"]
      }
    ],
    // [8]
    "Healthcheck": {
      "Command": {
        "Executable": "komorebic.exe",
        "Arguments": ["state"],
        "DelaySec": 1
      }
    },
    "Restart": "Never"
  }
}
```

{% endraw %}
