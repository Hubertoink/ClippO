param([ValidateSet('dev','build','check','test')][string]$Action = 'dev')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
$vc = Get-ChildItem "${env:ProgramFiles(x86)}\Microsoft Visual Studio\*\*\VC\Auxiliary\Build\vcvars64.bat","${env:ProgramFiles}\Microsoft Visual Studio\*\*\VC\Auxiliary\Build\vcvars64.bat" -ErrorAction SilentlyContinue | Select-Object -First 1
if ($vc) {
  $variables = & cmd.exe /d /c "call `"$($vc.FullName)`" >nul && set"
  foreach ($entry in $variables) {
    if ($entry -match '^([^=]+)=(.*)$') { [Environment]::SetEnvironmentVariable($matches[1],$matches[2],'Process') }
  }
}
& "$PSScriptRoot\engines.ps1"
if ($Action -in 'check','test') { cargo $Action --manifest-path src-tauri/Cargo.toml }
else { & "$root\node_modules\.bin\tauri.cmd" $Action }
exit $LASTEXITCODE
