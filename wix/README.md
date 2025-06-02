## Instructions

1. Preparation:

   - Make sure Wix Toolset 6.0 is installed: `dotnet tool install --global wix --version 6.0.0`

2. Build MSI:

   - For x64 architecture: `dotnet build -c Release -p:Platform=x64`
   - For ARM64 architecture: `dotnet build -c Release -p:Platform=arm64`

3. Installation Options:

   - User scope installation: `winget install Nushell.Nushell`
   - Machine scope installation: `winget install Nushell.Nushell --override 'ALLUSERS=1'` (requires administrator privileges)

   # For Per-User Installation
   `msiexec /i bin\x64\Release\nu-x64.msi /quiet /qn`

   # For Per-Machine Installation (Requires Admin Privileges)
   `msiexec /i bin\x64\Release\nu-x64.msi ALLUSERS=1`

   # MSI Install with Logs
   `msiexec /i bin\x64\Release\nu-x64.msi ALLUSERS=1 /l*v log.txt`

## Features

1. Dual installation scope: Supports both user and machine scope installation
2. `PATH` environment variable: Automatically adds the installation directory to the system `PATH`
3. Upgrade retention: Retains the original installation path during upgrades
4. Multi-architecture support: Supports `x86_64` and `ARM64` architectures
5. System compatibility: Compatible with Windows 7/10/11

## User-Facing Changes

- Nushell should be possible to be installed via winget with both user and machine scope and the default should be user scope
  - User scope install by winget: `winget install Nushell.Nushell`
  - User scope install by msiexec: `msiexec /i nu-0.104.1-x86_64-pc-windows-msvc.msi /quiet /qn`
  - Machine scope install by winget: `winget install Nushell.Nushell --override 'ALLUSERS=1'`
  - Machine scope install by msiexec: `msiexec /i nu-0.104.1-x86_64-pc-windows-msvc.msi ALLUSERS=1`
  - Note that the `--scope` flag for `winget install` does not work, use `--override` instead
  - Default user scope install dir: `$'($nu.home-path)\AppData\Local\Programs\nu\'`
  - Default machine scope install dir: `C:\Program Files\nu\`
- When a standard user runs the installer and selects "Install for everyone" (per-machine installation), Windows will automatically trigger a UAC prompt, the user can enter admin credentials and the installation will proceed with elevated privileges

## Test Case

- Install Nushell for the current user:
  - Check if the default installation path is correct
  - Check if silent installation: `msiexec /i $pkg MSIINSTALLPERUSER=1 /quiet /qn` works properly
  - Check if silent installation: `winget install --manifest manifests\n\Nushell\Nushell\0.104.1 --scope user` works properly
  - Ensure no UAC prompt appears during installation
  - Check if the environment variable is correctly added after installation
  - Check if the registry variable is correctly set
  - Check if the Windows Terminal configuration file is added
  - If the Windows Terminal configuration Feature is not selected, it should not be installed
  - If it is an upgrade installation, check if the original installation path is retained
  - Uninstall Nushell and check if files/environment variables/registry/Windows Terminal configuration files are cleaned up

- Install Nushell for all users
  - If a standard user installs, check if a UAC prompt appears during installation
  - Check if the default installation path is correct
  - Check if silent installation: `winget install --manifest manifests\n\Nushell\Nushell\0.104.1 --scope machine` works properly
  - Check if users are allowed to choose a custom installation path
  - Check if choosing a custom installation path allows successful installation
  - If it is an upgrade installation, check if the original installation path is retained
  - Check if the environment variable is correctly added after installation
  - Check if the registry variable is correctly set
  - Check if the Windows Terminal configuration file is added
  - If the Windows Terminal configuration Feature is not selected, it should not be installed
  - Uninstall Nushell and check if files/environment variables/registry/Windows Terminal configuration files are cleaned up

## REF

- https://docs.firegiant.com/quick-start/
- https://docs.firegiant.com/wix/schema/wxs/package/
- https://learn.microsoft.com/en-ca/windows/win32/msi/formatted
- https://learn.microsoft.com/en-us/windows/win32/msi/single-package-authoring
- https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/msiexec
