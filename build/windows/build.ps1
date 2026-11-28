param([string]$Version = "")

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "../..")

if (-not $Version) {
    $match = Select-String -Path Cargo.toml -Pattern '^\s*version = "([^"]+)"' | Select-Object -First 1
    if (-not $match) { throw "Cannot read version from Cargo.toml" }
    $Version = $match.Matches.Groups[1].Value
}

Write-Host "Building CYS $Version for Windows..."
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$issc = (Get-Command iscc -ErrorAction SilentlyContinue).Source
if (-not $issc) {
    foreach ($candidate in @(
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
    )) {
        if ($candidate -and (Test-Path $candidate)) {
            $issc = $candidate
            break
        }
    }
}

if (-not $issc) {
    Write-Warning "Inno Setup 6 not found. Install it from https://jrsoftware.org/isinfo.php and run:"
    Write-Warning "  iscc build\windows\cys.iss /DMyAppVersion=$Version"
    exit 0
}

New-Item -ItemType Directory -Force -Path dist | Out-Null
& $issc "build\windows\cys.iss" "/DMyAppVersion=$Version"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "Installer: dist\CYS-$Version-setup.exe"
