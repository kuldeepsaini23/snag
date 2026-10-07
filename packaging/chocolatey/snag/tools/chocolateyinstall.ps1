$ErrorActionPreference = 'Stop'

$packageArgs = @{
  packageName    = $env:ChocolateyPackageName
  fileType       = 'exe'
  url64bit       = 'https://github.com/kuldeepsaini23/snag/releases/download/v1.0.1/Snag-Setup-1.0.1.exe'
  checksum64     = '5E0E1444C4DC731BC120140214584E24859378EF5FB5A2DB3B0774FB3D49DACF'
  checksumType64 = 'sha256'
  # Inno Setup: no windows, no reboot, no "open Snag now".
  silentArgs     = '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-'
  validExitCodes = @(0)
  softwareName   = 'Snag*'
}

Install-ChocolateyPackage @packageArgs
