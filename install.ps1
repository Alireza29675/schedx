$ErrorActionPreference = "Stop"

$Repo = if ($env:SCHEDX_REPO) { $env:SCHEDX_REPO } else { "Alireza29675/schedx" }
$InstallDir = if ($env:SCHEDX_INSTALL_DIR) {
    $env:SCHEDX_INSTALL_DIR
} else {
    Join-Path $env:USERPROFILE "AppData\Local\Programs\schedx\bin"
}

function Get-Target {
    switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
        "X64" { return "x86_64-pc-windows-msvc" }
        "Arm64" { throw "Windows ARM64 release artifacts are not published yet." }
        default { throw "Unsupported Windows architecture: $([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture)" }
    }
}

$Target = Get-Target
$Version = if ($env:SCHEDX_VERSION) {
    $env:SCHEDX_VERSION
} else {
    (Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest").tag_name
}

if (-not $Version) {
    throw "Could not determine the latest schedx version."
}

$Tarball = "schedx-$Target.tar.gz"
$BaseUrl = "https://github.com/$Repo/releases/download/$Version"
$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("schedx-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $TempDir | Out-Null

try {
    Write-Host "Installing schedx $Version for $Target..."

    $TarballPath = Join-Path $TempDir $Tarball
    $ChecksumsPath = Join-Path $TempDir "checksums.txt"

    Invoke-WebRequest -Uri "$BaseUrl/$Tarball" -OutFile $TarballPath
    Invoke-WebRequest -Uri "$BaseUrl/checksums.txt" -OutFile $ChecksumsPath

    $ExpectedLine = Select-String -Path $ChecksumsPath -Pattern [regex]::Escape($Tarball) | Select-Object -First 1
    $Expected = if ($ExpectedLine) { ($ExpectedLine.Line -split "\s+")[0] } else { "" }
    if (-not $Expected) {
        throw "Checksum not found for $Tarball."
    }

    $Actual = (Get-FileHash -Algorithm SHA256 -Path $TarballPath).Hash.ToLowerInvariant()
    if ($Expected.ToLowerInvariant() -ne $Actual) {
        throw "Checksum mismatch for $Tarball."
    }

    tar -xzf $TarballPath -C $TempDir
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -Force (Join-Path $TempDir "schedx.exe") (Join-Path $InstallDir "schedx.exe")

    Write-Host "Installed schedx to $(Join-Path $InstallDir 'schedx.exe')"
}
finally {
    Remove-Item -Recurse -Force $TempDir
}
