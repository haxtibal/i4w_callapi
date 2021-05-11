# Icinga for Windows REST API calling tool

[![Build status](https://github.com/haxtibal/i4w_callapi/actions/workflows/ci.yml/badge.svg)](https://github.com/haxtibal/i4w_callapi/actions)

This is a lean alternative to the [Icinga for Windows](https://icinga.com/docs/icinga-for-windows/latest/) `Exit-IcingaExecutePlugin` PowerShell REST API client.
It's useful to get rid of the performance penalty introduced by frequently calling powershell.exe from the agent.

## Background

Icinga for Windows provides a [icinga-powershell-framework](https://github.com/Icinga/icinga-powershell-framework) and related
[icinga-powershell-plugins](https://github.com/Icinga/icinga-powershell-plugins). Check plugins usually don't consume much resources on their own,
but spinning up powershell.exe and initializing the Icinga framework is expensive. In normal operation mode,
the Icinga agent would periodically execute each plugin as short lived child process. That multiplies the CPU time spent for no real use.

As mitigation, a separate long living [PowerShell host daemon](https://github.com/Icinga/icinga-powershell-restapi)
with [a REST endpoint for check execution](https://github.com/Icinga/icinga-powershell-apichecks) was invented by the Icinga team,
where initialization is done only once, and plugins are then called as function rather than started as new process.

The agent would now just perform a cheap local HTTP POST to the daemon. In pracitce however it's not so cheap,
because Icinga for Windows suggests to use yet another PowerShell function `Exit-IcingaExecutePlugin` to perform the HTTP POST,
which basically reintroduces above problems.

This is where i4w_callapi helps: Get completely rid of PowerShell at agent side.
Replace it with something that is optimized to safe resouces.

## Installation

Preparation: Setup Icinga for Windows and enable it for REST API checks. Not documented here.

Compile - you can statically link the C runtime and get a self-contained binary for easy distribution.
```
cargo build --target=x86_64-pc-windows-msvc -Ctarget-feature=+crt-static --release
```

Copy the binary into the agents bin path, which defaults to `C:\Program Files\ICINGA2\sbin`.

## Usage

Manual invokation example
```
> call_api_check.exe -c Invoke-IcingaCheckCPU -- -Warning 50 -Critical 90
[OK] Check package "CPU Load" | 'core_4'=3.378296%;50;90;0;100 'core_total'=3.948704%;50;90;0;100 'core_6'=3.345666%;50;
90;0;100 'core_5'=3.39417%;50;90;0;100 'core_7'=3.038184%;50;90;0;100 'core_0'=5.790892%;50;90;0;100 'core_2'=4.960417%;
50;90;0;100 'core_1'=3.999451%;50;90;0;100 'core_3'=3.682836%;50;90;0;100
```

In operation, the executable is intended to be used from a `object CheckCommand` definition.
```
object CheckCommand "PowerShell Base" {
   import "plugin-check-command"
   command = [ PluginDir + "/call_api_check.exe" ]
   arguments = {
       "--insecure" = {
            order = -3
            set_if = true
       }
       "--" = {
            order = -1
            set_if = true
        }
   }
   timeout = 3m
}

object CheckCommand "Invoke-IcingaCheckCPU" {
    import "PowerShell Base"
    arguments += {
        "-c" = {
            order = -2
            value = "Invoke-IcingaCheckCPU"
        }
        "-Warning" = {
            order = 1
            value = "$IcingaCheckCPU_Object_Warning$"
        }
        "-Critical" = {
            order = 2
            value = "$IcingaCheckCPU_Object_Critical$"
        }
        "-Core" = {
            order = 3
            value = "$IcingaCheckCPU_String_Core$"
        }
        "-NoPerfData" = {
            order = 99
            set_if = "$IcingaCheckCPU_SwitchParameter_NoPerfData$"
        }
        "-Verbosity" = {
            order = 4
            value = "$IcingaCheckCPU_Int32_Verbosity$"
        }
    }
    vars.IcingaCheckCPU_Object_Warning = "$$null"
    vars.IcingaCheckCPU_Object_Critical = "$$null"
    vars.IcingaCheckCPU_String_Core = "*"
    vars.IcingaCheckCPU_SwitchParameter_NoPerfData = false
    vars.IcingaCheckCPU_Int32_Verbosity = 0
}
```