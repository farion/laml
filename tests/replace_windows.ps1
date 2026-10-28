# Smoke-test the Windows post-exit replacement helper without changing the running binary.
$ErrorActionPreference = 'Stop'
$work = Join-Path ([IO.Path]::GetTempPath()) ("laml-windows-update-test-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
try {
    $destination = Join-Path $work 'laml.exe'
    $staged = Join-Path $work 'staged.exe'
    $helper = Join-Path $work 'replace.ps1'
    [IO.File]::WriteAllText($destination, 'old')
    [IO.File]::WriteAllText($staged, 'new')
    Copy-Item scripts/replace-windows.ps1 $helper
    & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $helper `
        -ParentPid 999999 -Staged $staged -Destination $destination -Helper $helper
    if ($LASTEXITCODE -ne 0 -or [IO.File]::ReadAllText($destination) -ne 'new') {
        throw 'Windows update helper did not replace the staged executable'
    }
    if (Test-Path -LiteralPath "$destination.update-error.txt") {
        throw 'Windows update helper reported an error'
    }
} finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Output 'Windows update helper smoke test passed'
