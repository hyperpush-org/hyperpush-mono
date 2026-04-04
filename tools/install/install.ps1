# Install script for meshc and meshpkg - the Mesh CLI tools (Windows)
# Usage: powershell -ExecutionPolicy ByPass -c "irm https://meshlang.dev/install.ps1 | iex"
# Or: .\install.ps1 [-Version VERSION] [-Uninstall] [-Yes] [-Help]

param(
    [string]$Version = "",
    [switch]$Uninstall,
    [switch]$Yes,
    [switch]$Help
)

$ErrorActionPreference = 'Stop'

$Repo = "snowdamiz/mesh-lang"
$MeshHome = "$env:USERPROFILE\.mesh"
$BinDir = "$MeshHome\bin"
$VersionFile = "$MeshHome\version"

# --- Color output ---

function Use-Color {
    if ($env:NO_COLOR) { return $false }
    return $true
}

function Say {
    param([string]$Message)
    Write-Host $Message
}

function Say-Green {
    param([string]$Message)
    if (Use-Color) {
        Write-Host $Message -ForegroundColor Green
    } else {
        Write-Host $Message
    }
}

function Say-Red {
    param([string]$Message)
    if (Use-Color) {
        Write-Host $Message -ForegroundColor Red
    } else {
        Write-Host $Message
    }
}

function Show-ErrorMessage {
    param([string]$Message)
    foreach ($line in ($Message -split "`r?`n")) {
        if ($line -ne '') {
            Say-Red $line
        }
    }
}

function Fail-Installer {
    param([string]$Message)
    throw $Message
}

function Test-InstallerBool {
    param([string]$Value)
    if (-not $Value) { return $false }
    switch ($Value.ToLowerInvariant()) {
        '1' { return $true }
        'true' { return $true }
        'yes' { return $true }
        'on' { return $true }
        default { return $false }
    }
}

function Get-ReleaseApiUrl {
    if ($env:MESH_INSTALL_RELEASE_API_URL) {
        return $env:MESH_INSTALL_RELEASE_API_URL
    }
    return "https://api.github.com/repos/$Repo/releases/latest"
}

function Get-ReleaseBaseUrl {
    if ($env:MESH_INSTALL_RELEASE_BASE_URL) {
        return $env:MESH_INSTALL_RELEASE_BASE_URL.TrimEnd('/')
    }
    return "https://github.com/$Repo/releases/download"
}

function Get-DownloadTimeoutSec {
    $parsed = 0
    if ($env:MESH_INSTALL_DOWNLOAD_TIMEOUT_SEC -and [int]::TryParse($env:MESH_INSTALL_DOWNLOAD_TIMEOUT_SEC, [ref]$parsed) -and $parsed -gt 0) {
        return $parsed
    }
    return 120
}

function Test-StrictProofMode {
    return Test-InstallerBool $env:MESH_INSTALL_STRICT_PROOF
}

function Get-RequestHeaders {
    return @{ "User-Agent" = "mesh-installer" }
}

# --- Platform detection ---

function Detect-Architecture {
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    switch ($arch) {
        "X64" { return "x86_64" }
        default {
            Fail-Installer "error: Unsupported architecture: $arch`n  meshc currently supports x86_64 (64-bit) Windows only."
        }
    }
}

# --- Version management ---

function Get-LatestVersion {
    $releaseApiUrl = Get-ReleaseApiUrl
    try {
        $release = Invoke-RestMethod -Uri $releaseApiUrl -Headers (Get-RequestHeaders) -TimeoutSec (Get-DownloadTimeoutSec)
    } catch {
        Fail-Installer "error: Failed to fetch release metadata.`n  URL: $releaseApiUrl"
    }

    $tagProperty = $release.PSObject.Properties['tag_name']
    $tagName = if ($tagProperty) { [string]$tagProperty.Value } else { '' }
    $version = $tagName -replace '^v', ''
    if (-not $version) {
        Fail-Installer "error: Release metadata did not contain tag_name.`n  URL: $releaseApiUrl"
    }

    return $version
}

function Check-UpdateNeeded {
    param([string]$TargetVersion)

    if (Test-Path $VersionFile) {
        $current = (Get-Content $VersionFile -Raw).Trim()
        if ($current -eq $TargetVersion) {
            Say-Green "meshc and meshpkg v$TargetVersion are already installed and up-to-date."
            return $false
        }
        Say "Updating meshc and meshpkg from v$current to v$TargetVersion..."
    }
    return $true
}

# --- Checksum verification ---

function Verify-Checksum {
    param(
        [string]$FilePath,
        [string]$Expected
    )

    $actual = (Get-FileHash -Path $FilePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $Expected.ToLowerInvariant()) {
        Fail-Installer "error: Checksum verification failed.`n  archive:  $FilePath`n  expected: $Expected`n  actual:   $actual"
    }
}

function Get-ExpectedChecksum {
    param(
        [string]$ChecksumPath,
        [string]$ArchiveName
    )

    $line = Get-Content $ChecksumPath | Where-Object { $_ -match ("\s+" + [regex]::Escape($ArchiveName) + '$') } | Select-Object -First 1
    if (-not $line) {
        return $null
    }

    $parts = $line -split '\s+'
    if ($parts.Count -lt 2) {
        return '__MALFORMED__'
    }

    $hash = $parts[0]
    if ($hash -notmatch '^[0-9A-Fa-f]{64}$') {
        return '__MALFORMED__'
    }

    return $hash
}

function Invoke-MeshDownload {
    param(
        [string]$Url,
        [string]$OutFile,
        [string]$FailureMessage
    )

    try {
        Invoke-WebRequest -Uri $Url -OutFile $OutFile -Headers (Get-RequestHeaders) -TimeoutSec (Get-DownloadTimeoutSec) | Out-Null
    } catch {
        Fail-Installer "$FailureMessage`n  URL: $Url`n  timeout: $(Get-DownloadTimeoutSec)s"
    }
}

# --- Uninstall ---

function Invoke-Uninstall {
    Say "Uninstalling meshc and meshpkg..."

    if (Test-Path $MeshHome) {
        Remove-Item -Recurse -Force $MeshHome
    }

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -and $userPath -like "*$BinDir*") {
        $parts = $userPath -split ';' | Where-Object { $_ -ne $BinDir -and $_ -ne "" }
        [Environment]::SetEnvironmentVariable("Path", ($parts -join ';'), "User")
    }

    Say-Green "meshc and meshpkg have been uninstalled."
}

# --- Install helpers ---

function Install-Binary {
    param(
        [string]$BinaryName,
        [string]$RequestedVersion,
        [string]$Target
    )

    $archive = "$BinaryName-v${RequestedVersion}-${Target}.zip"
    $baseUrl = Get-ReleaseBaseUrl
    $url = "$baseUrl/v${RequestedVersion}/$archive"
    $tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "mesh-install-$([System.Guid]::NewGuid().ToString('N').Substring(0, 8))"
    $archivePath = Join-Path $tmpDir $archive
    $checksumUrl = "$baseUrl/v${RequestedVersion}/SHA256SUMS"
    $checksumPath = Join-Path $tmpDir "SHA256SUMS"
    $success = $false

    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

    try {
        Say "Installing $BinaryName v$RequestedVersion ($Target)..."
        Invoke-MeshDownload -Url $url -OutFile $archivePath -FailureMessage "error: Failed to download $BinaryName v$RequestedVersion."

        try {
            Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumPath -Headers (Get-RequestHeaders) -TimeoutSec (Get-DownloadTimeoutSec) | Out-Null
        } catch {
            if (Test-StrictProofMode) {
                Fail-Installer "error: Could not download SHA256SUMS in staged-proof mode.`n  URL: $checksumUrl`n  timeout: $(Get-DownloadTimeoutSec)s"
            }
            Say "warning: Could not download SHA256SUMS, skipping checksum verification."
        }

        if (Test-Path $checksumPath) {
            $expectedHash = Get-ExpectedChecksum -ChecksumPath $checksumPath -ArchiveName $archive
            if ($expectedHash -eq '__MALFORMED__') {
                if (Test-StrictProofMode) {
                    Fail-Installer "error: SHA256SUMS contained a malformed checksum for $archive.`n  checksum file: $checksumPath`n  checksum URL:  $checksumUrl"
                }
                Say "warning: Malformed SHA256SUMS entry for $archive, skipping verification."
            } elseif ($expectedHash) {
                Verify-Checksum -FilePath $archivePath -Expected $expectedHash
            } else {
                if (Test-StrictProofMode) {
                    Fail-Installer "error: SHA256SUMS did not contain $archive.`n  checksum file: $checksumPath`n  checksum URL:  $checksumUrl"
                }
                Say "warning: Archive not found in SHA256SUMS, skipping verification."
            }
        }

        $extractDir = Join-Path $tmpDir 'extracted'
        try {
            Expand-Archive -Path $archivePath -DestinationPath $extractDir -Force
        } catch {
            Fail-Installer "error: Failed to extract $archive.`n  archive: $archivePath"
        }

        $sourceBinary = Get-ChildItem -Path $extractDir -Filter "$BinaryName.exe" -Recurse | Select-Object -First 1
        if (-not $sourceBinary) {
            Fail-Installer "error: $BinaryName.exe was not found after extracting $archive.`n  archive: $archivePath"
        }

        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
        Copy-Item -Path $sourceBinary.FullName -Destination (Join-Path $BinDir "$BinaryName.exe") -Force
        $success = $true
    } finally {
        if ($success -and (Test-Path $tmpDir)) {
            Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
        }
    }
}

# --- Install ---

function Invoke-Install {
    param([string]$RequestedVersion)

    try {
        if (-not $RequestedVersion) {
            Say "Fetching latest version..."
            $RequestedVersion = Get-LatestVersion
        }

        if (-not (Check-UpdateNeeded -TargetVersion $RequestedVersion)) {
            return
        }

        $arch = Detect-Architecture
        $target = "${arch}-pc-windows-msvc"

        Install-Binary -BinaryName 'meshc' -RequestedVersion $RequestedVersion -Target $target
        Install-Binary -BinaryName 'meshpkg' -RequestedVersion $RequestedVersion -Target $target

        New-Item -ItemType Directory -Path $MeshHome -Force | Out-Null
        Set-Content -Path $VersionFile -Value $RequestedVersion -NoNewline

        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        if (-not $userPath -or $userPath -notlike "*$BinDir*") {
            if ($userPath) {
                [Environment]::SetEnvironmentVariable('Path', "$BinDir;$userPath", 'User')
            } else {
                [Environment]::SetEnvironmentVariable('Path', $BinDir, 'User')
            }
        }

        Say-Green "Installed meshc and meshpkg v$RequestedVersion to ~\.mesh\bin\"
        Say "Run 'meshc --version' and 'meshpkg --version' to verify, or restart your terminal."
    } catch {
        Show-ErrorMessage $_.Exception.Message
        exit 1
    }
}

# --- Usage ---

function Show-Usage {
    Say "Mesh installer (Windows)"
    Say ""
    Say "Usage: install.ps1 [OPTIONS]"
    Say ""
    Say "Options:"
    Say "  -Version VERSION  Install a specific version (default: latest)"
    Say "  -Uninstall        Remove meshc and meshpkg and clean up PATH changes"
    Say "  -Yes              Accept defaults (for CI, already non-interactive)"
    Say "  -Help             Show this help message"
}

# --- Main ---

if ($Help) {
    Show-Usage
    exit 0
}

if ($Uninstall) {
    Invoke-Uninstall
} else {
    Invoke-Install -RequestedVersion $Version
}
