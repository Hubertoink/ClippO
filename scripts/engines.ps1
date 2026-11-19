$ErrorActionPreference = 'Stop'
$dest = Join-Path (Split-Path $PSScriptRoot -Parent) 'src-tauri/resources/engines'
$cache = Join-Path $env:LOCALAPPDATA 'ClippO/build-cache'
New-Item -ItemType Directory -Force $cache,$dest | Out-Null
function Get-Archive($name,$url,$hash) {
  $archive = Join-Path $cache $name
  if (!(Test-Path $archive) -or (Get-FileHash $archive).Hash -ne $hash) {
    & curl.exe --fail --silent --show-error --location --retry 2 -o "$archive.part" $url
    if ($LASTEXITCODE -ne 0 -or (Get-FileHash "$archive.part").Hash -ne $hash) { throw "Download verification failed for $name" }
    Move-Item -Force -LiteralPath "$archive.part" -Destination $archive
  }
  $target = Join-Path $cache "$name.extracted"
  New-Item -ItemType Directory -Force $target | Out-Null
  & tar.exe -xf $archive -C $target
  if ($LASTEXITCODE -ne 0) { throw "Could not extract $name" }
  return $target
}
if (!(Test-Path "$dest/qpdf/qpdf.exe")) {
  $q = Get-Archive 'qpdf-12.4.2.zip' 'https://github.com/qpdf/qpdf/releases/download/v12.4.2/qpdf-12.4.2-msvc64.zip' 'db87077e683630c1217e0e8f9a20a9749d952ab676e881c3689187763a5de25d'
  $exe = Get-ChildItem $q -Recurse -Filter qpdf.exe | Select-Object -First 1
  New-Item -ItemType Directory -Force "$dest/qpdf" | Out-Null
  Copy-Item "$($exe.DirectoryName)/*.dll" "$dest/qpdf/"
  Get-ChildItem $q -Recurse -File -Include 'Artistic*','LICENSE*','NOTICE*' | Copy-Item -Destination "$dest/qpdf/" -Force
  Copy-Item $exe.FullName "$dest/qpdf/"
}
if (!(Test-Path "$dest/magick/magick.exe")) {
  $m = Get-Archive 'magick-7.1.2-36.7z' 'https://github.com/ImageMagick/ImageMagick/releases/download/7.1.2-36/ImageMagick-7.1.2-36-portable-Q8-x64.7z' '90e3a8ee011710cf4e570c040eaf430daf6e69b1fab084fc3214cb695f961bdc'
  $exe = Get-ChildItem $m -Recurse -Filter magick.exe | Select-Object -First 1
  New-Item -ItemType Directory -Force "$dest/magick" | Out-Null
  Get-ChildItem $exe.DirectoryName -File -Filter '*.xml' | Copy-Item -Destination "$dest/magick/"
  Get-ChildItem $exe.DirectoryName -File | Where-Object Name -match 'LICENSE|NOTICE|\.dll$' | Copy-Item -Destination "$dest/magick/"
  Copy-Item $exe.FullName "$dest/magick/"
}
if (!(Test-Path "$dest/magick/configure.xml")) {
  Get-ChildItem "$cache/magick-7.1.2-36.7z.extracted" -File -Filter '*.xml' | Copy-Item -Destination "$dest/magick/"
}
