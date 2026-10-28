# Internal helper embedded in laml.exe; runs after the parent process exits.
param(
    [Parameter(Mandatory=$true)][int]$ParentPid,
    [Parameter(Mandatory=$true)][string]$Staged,
    [Parameter(Mandatory=$true)][string]$Destination,
    [Parameter(Mandatory=$true)][string]$Helper
)
$ErrorActionPreference = 'Stop'
$backup = "$Destination.previous-$ParentPid"
try {
    $deadline = (Get-Date).AddSeconds(30)
    while (Get-Process -Id $ParentPid -ErrorAction SilentlyContinue) {
        if ((Get-Date) -gt $deadline) { throw 'Timed out waiting for LAML to exit' }
        Start-Sleep -Milliseconds 200
    }
    if (Test-Path -LiteralPath $Destination) { Move-Item -LiteralPath $Destination -Destination $backup }
    try { Move-Item -LiteralPath $Staged -Destination $Destination }
    catch {
        if (Test-Path -LiteralPath $backup) { Move-Item -LiteralPath $backup -Destination $Destination }
        throw
    }
    if (Test-Path -LiteralPath $backup) { Remove-Item -LiteralPath $backup -Force }
    if (Test-Path -LiteralPath "$Destination.update-error.txt") {
        Remove-Item -LiteralPath "$Destination.update-error.txt" -Force
    }
} catch {
    $_ | Out-String | Set-Content -LiteralPath "$Destination.update-error.txt" -Encoding UTF8
} finally {
    if (Test-Path -LiteralPath $Staged) { Remove-Item -LiteralPath $Staged -Force }
    Remove-Item -LiteralPath $Helper -Force -ErrorAction SilentlyContinue
}
