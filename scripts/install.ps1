# Install the latest published LAML release for Windows (PowerShell 5.1+).
param([string]$InstallDir)
$ErrorActionPreference = 'Stop'
if (-not $InstallDir) { $InstallDir = $env:LAML_INSTALL_DIR }
if (-not $InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\laml' }
if ([Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() -ne 'X64') {
    throw 'Only Windows x86-64 release binaries are available'
}
$headers = @{ Accept = 'application/vnd.github+json'; 'User-Agent' = 'laml-installer' }
$release = Invoke-RestMethod -Uri 'https://api.github.com/repos/farion/laml/releases/latest' -Headers $headers
$tag = [string]$release.tag_name
if ($tag -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+$') { throw 'No published stable LAML release found' }
$name = "laml-$tag-x86_64-pc-windows-msvc"
$archiveName = "$name.zip"
$base = "https://github.com/farion/laml/releases/download/$tag"
$work = Join-Path ([IO.Path]::GetTempPath()) ("laml-install-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
$staged = $null
try {
    Invoke-WebRequest -Uri "$base/$archiveName" -OutFile (Join-Path $work $archiveName) -UseBasicParsing
    Invoke-WebRequest -Uri "$base/SHA256SUMS" -OutFile (Join-Path $work 'SHA256SUMS') -UseBasicParsing
    $pattern = '^([0-9a-fA-F]{64})\s+\*?' + [regex]::Escape($archiveName) + '$'
    $line = Get-Content (Join-Path $work 'SHA256SUMS') | Where-Object { $_ -match $pattern } | Select-Object -First 1
    if (-not $line) { throw 'Missing or invalid archive checksum' }
    $expected = [regex]::Match($line, $pattern).Groups[1].Value
    $actual = (Get-FileHash (Join-Path $work $archiveName) -Algorithm SHA256).Hash
    if ($actual -ne $expected) { throw 'Archive checksum mismatch; nothing installed' }
    Expand-Archive -LiteralPath (Join-Path $work $archiveName) -DestinationPath (Join-Path $work 'unpacked')
    $binary = Join-Path $work "unpacked/$name/laml.exe"
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw 'Archive contains no LAML binary' }
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    $staged = Join-Path $InstallDir ('.laml-' + [guid]::NewGuid().ToString('N') + '.exe')
    Copy-Item -LiteralPath $binary -Destination $staged
    $destination = Join-Path $InstallDir 'laml.exe'
    if (Test-Path -LiteralPath $destination) {
        $backup = Join-Path $InstallDir ('.laml-backup-' + [guid]::NewGuid().ToString('N') + '.exe')
        Move-Item -LiteralPath $destination -Destination $backup
        try { Move-Item -LiteralPath $staged -Destination $destination; $staged = $null }
        catch { Move-Item -LiteralPath $backup -Destination $destination; throw }
        Remove-Item -LiteralPath $backup -Force
    } else {
        Move-Item -LiteralPath $staged -Destination $destination
        $staged = $null
    }
    $userPath = [string][Environment]::GetEnvironmentVariable('Path', 'User')
    if (($userPath -split ';') -notcontains $InstallDir) {
        $newPath = if ($userPath) { $userPath.TrimEnd(';') + ';' + $InstallDir } else { $InstallDir }
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        Write-Output 'Added LAML to user PATH. Open a new terminal to run it.'
    }
    Write-Output "Installed LAML $tag to $destination"
} finally {
    if ($staged -and (Test-Path -LiteralPath $staged)) { Remove-Item -LiteralPath $staged -Force }
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
