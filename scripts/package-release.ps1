param([string]$TargetDir = $env:CARGO_TARGET_DIR, [switch]$Signed)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
if (!$TargetDir) { $TargetDir = Join-Path $root 'src-tauri/target' }
$version = (Get-Content (Join-Path $root 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
$name = "ClippO_${version}_x64-setup.exe"
$binaryDir = Join-Path $TargetDir 'release'
$installer = Join-Path $binaryDir "bundle/nsis/$name"
$output = Join-Path $root 'release'
New-Item $output -ItemType Directory -Force | Out-Null
Copy-Item -LiteralPath $installer -Destination $output

$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$stage = Join-Path $tempRoot ("clippo-portable-" + [guid]::NewGuid())
New-Item -Path $stage -ItemType Directory | Out-Null
try {
  Copy-Item -LiteralPath (Join-Path $binaryDir 'clippo.exe') -Destination $stage
  Copy-Item -LiteralPath (Join-Path $root 'src-tauri/resources') -Destination $stage -Recurse
  New-Item -Path (Join-Path $stage 'clippo-portable') -ItemType File | Out-Null
  Compress-Archive -Path (Join-Path $stage '*') -DestinationPath (Join-Path $output "ClippO_${version}_x64-portable.zip") -Force
} finally {
  $resolved = (Resolve-Path -LiteralPath $stage).Path
  if ((Split-Path $resolved -Parent) -eq $tempRoot.TrimEnd('\') -and (Split-Path $resolved -Leaf) -match '^clippo-portable-[0-9a-f-]{36}$') {
    Remove-Item -LiteralPath $resolved -Recurse -Force
  }
}

if ($Signed) {
  $signature = (Get-Content -LiteralPath "$installer.sig" -Raw).Trim()
  if (!$signature) { throw 'Missing updater signature' }
  Copy-Item -LiteralPath "$installer.sig" -Destination $output
  $manifest = [ordered]@{
    version = $version
    pub_date = [DateTime]::UtcNow.ToString('o')
    platforms = @{ 'windows-x86_64' = @{
      signature = $signature
      url = "https://github.com/Hubertoink/ClippO/releases/download/v$version/$name"
    } }
  }
  [IO.File]::WriteAllText((Join-Path $output 'latest.json'), ($manifest | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false))
}
