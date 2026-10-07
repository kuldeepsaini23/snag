$ErrorActionPreference = 'Stop'

$packageArgs = @{
  packageName    = $env:ChocolateyPackageName
  fileType       = 'exe'
  url64bit       = 'https://github.com/kuldeepsaini23/snag/releases/download/v1.0.0/Snag-Setup-1.0.0.exe'
  checksum64     = 'A32C9F0C7E85613FAC3C87613BF1BFD6AE3999CCFF66E7D14B40FAE389B5C50B'
  checksumType64 = 'sha256'
  # Inno Setup: no windows, no reboot, no "open Snag now".
  silentArgs     = '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-'
  validExitCodes = @(0)
  softwareName   = 'Snag*'
}

Install-ChocolateyPackage @packageArgs
