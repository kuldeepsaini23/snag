$ErrorActionPreference = 'Stop'

# Settings and download history are kept (the uninstaller's question defaults to "No" when silent);
# downloaded files are never touched.
[array]$keys = Get-UninstallRegistryKey -SoftwareName 'Snag*'
foreach ($key in $keys) {
  $uninstaller = ($key.UninstallString -replace '"', '')
  Uninstall-ChocolateyPackage -PackageName $env:ChocolateyPackageName -FileType 'exe' `
    -SilentArgs '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' -File $uninstaller -ValidExitCodes @(0)
}
