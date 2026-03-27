$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RootDir = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $RootDir

$TmpRoot = Join-Path $RootDir '.tmp/m034-s03/windows'
$VerifyRoot = Join-Path $TmpRoot 'verify'
$StageRoot = Join-Path $TmpRoot 'stage'
$ServerRoot = Join-Path $StageRoot 'server'
$HomeRoot = Join-Path $TmpRoot 'home'
$WorkRoot = Join-Path $TmpRoot 'work'
$FixtureDir = Join-Path $RootDir 'scripts/fixtures/m034-s03-installer-smoke'
$InstallScript = Join-Path $RootDir 'website/docs/public/install.ps1'
$RepoInstallScript = Join-Path $RootDir 'tools/install/install.ps1'
$MeshcExe = Join-Path $RootDir 'target/debug/meshc.exe'
$MeshpkgExe = Join-Path $RootDir 'target/debug/meshpkg.exe'
$PrebuiltReleaseDir = $env:M034_S03_PREBUILT_RELEASE_DIR
$RunDir = Join-Path $VerifyRoot 'run'
$ServerProcess = $null
$LastStdoutPath = ''
$LastStderrPath = ''
$LastLogPath = ''
$Version = ''
$Target = 'x86_64-pc-windows-msvc'
$MeshcArchive = ''
$MeshpkgArchive = ''
$GoodRoot = Join-Path $ServerRoot 'good'

function Stop-LocalServer {
    if ($script:ServerProcess -and -not $script:ServerProcess.HasExited) {
        try {
            Stop-Process -Id $script:ServerProcess.Id -Force -ErrorAction SilentlyContinue
        } catch {
        }
    }
}

function Fail-Phase {
    param(
        [string]$Phase,
        [string]$Reason,
        [string]$LogPath = ''
    )

    Write-Error "verification drift: $Reason"
    Write-Error "first failing phase: $Phase"
    Write-Error "artifacts: $($script:RunDir)"
    Write-Error "staged root: $($script:ServerRoot)"
    if ($LogPath -and (Test-Path $LogPath)) {
        Write-Error "--- $LogPath ---"
        Get-Content $LogPath | Select-Object -First 260 | ForEach-Object { Write-Error $_ }
    }
    exit 1
}

function Combine-CommandLog {
    param(
        [string]$Display,
        [string]$StdoutPath,
        [string]$StderrPath,
        [string]$LogPath
    )

    $content = [System.Collections.Generic.List[string]]::new()
    $content.Add("display: $Display")
    if ((Test-Path $StdoutPath) -and (Get-Item $StdoutPath).Length -gt 0) {
        $content.Add('')
        $content.Add('[stdout]')
        $content.AddRange([string[]](Get-Content $StdoutPath))
    }
    if ((Test-Path $StderrPath) -and (Get-Item $StderrPath).Length -gt 0) {
        $content.Add('')
        $content.Add('[stderr]')
        $content.AddRange([string[]](Get-Content $StderrPath))
    }
    Set-Content -Path $LogPath -Value $content
}

function Invoke-LoggedCommand {
    param(
        [string]$Phase,
        [string]$Label,
        [string]$Display,
        [scriptblock]$Command,
        [switch]$ExpectFailure
    )

    $stdoutPath = Join-Path $script:RunDir "$Label.stdout"
    $stderrPath = Join-Path $script:RunDir "$Label.stderr"
    $logPath = Join-Path $script:RunDir "$Label.log"

    Write-Host "==> [$Phase] $Display"
    & $Command 1> $stdoutPath 2> $stderrPath
    $lastExitCodeVar = Get-Variable -Name LASTEXITCODE -Scope Global -ErrorAction SilentlyContinue
    if ($null -eq $lastExitCodeVar) {
        $exitCode = 0
    } else {
        $exitCode = $lastExitCodeVar.Value
        if ($null -eq $exitCode) { $exitCode = 0 }
    }

    Combine-CommandLog -Display $Display -StdoutPath $stdoutPath -StderrPath $stderrPath -LogPath $logPath
    $script:LastStdoutPath = $stdoutPath
    $script:LastStderrPath = $stderrPath
    $script:LastLogPath = $logPath

    if ($ExpectFailure) {
        if ($exitCode -eq 0) {
            Fail-Phase $Phase "$Display unexpectedly succeeded" $logPath
        }
        return
    }

    if ($exitCode -ne 0) {
        Fail-Phase $Phase "$Display failed" $logPath
    }
}

function Assert-LogContains {
    param(
        [string]$Phase,
        [string]$Needle,
        [string]$LogPath
    )

    if (-not (Select-String -Path $LogPath -SimpleMatch $Needle -Quiet)) {
        Fail-Phase $Phase "expected to find '$Needle' in $LogPath" $LogPath
    }
}

function Get-RepoVersion {
    $meshc = (Get-Content (Join-Path $RootDir 'compiler/meshc/Cargo.toml') -Raw) -match 'version = "([^"]+)"' | Out-Null
    $meshcVersion = $Matches[1]
    $meshpkg = (Get-Content (Join-Path $RootDir 'compiler/meshpkg/Cargo.toml') -Raw) -match 'version = "([^"]+)"' | Out-Null
    $meshpkgVersion = $Matches[1]
    if ($meshcVersion -ne $meshpkgVersion) {
        throw "meshc ($meshcVersion) and meshpkg ($meshpkgVersion) versions diverged"
    }
    return $meshcVersion
}

function Get-Sha256 {
    param([string]$Path)
    return (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-FreePort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $port = ($listener.LocalEndpoint).Port
    $listener.Stop()
    return $port
}

function Get-PythonCommand {
    foreach ($name in @('python', 'py')) {
        $cmd = Get-Command $name -ErrorAction SilentlyContinue
        if ($cmd) { return $cmd.Name }
    }
    throw 'python or py is required to host staged release assets'
}

function Find-SingleFile {
    param(
        [string]$Dir,
        [string]$Filter
    )

    $matches = @(Get-ChildItem -Path $Dir -Filter $Filter -File -ErrorAction SilentlyContinue)
    if ($matches.Count -ne 1) {
        throw "expected exactly one match for $Dir/$Filter, found $($matches.Count)"
    }

    return $matches[0].FullName
}

function Get-VersionFromArchiveName {
    param(
        [string]$Prefix,
        [string]$ArchiveName,
        [string]$Target,
        [string]$Extension
    )

    $expectedPrefix = "$Prefix-v"
    $expectedSuffix = "-$Target.$Extension"
    if (-not $ArchiveName.StartsWith($expectedPrefix) -or -not $ArchiveName.EndsWith($expectedSuffix)) {
        throw "could not infer version from $ArchiveName"
    }

    return $ArchiveName.Substring($expectedPrefix.Length, $ArchiveName.Length - $expectedPrefix.Length - $expectedSuffix.Length)
}

function New-ZipArchive {
    param(
        [string]$ArchivePath,
        [string]$SourcePath,
        [string]$EntryName
    )

    $tmpDir = Join-Path $StageRoot ("zip-" + [System.Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null
    Copy-Item $SourcePath (Join-Path $tmpDir $EntryName)
    Compress-Archive -Path (Join-Path $tmpDir $EntryName) -DestinationPath $ArchivePath -Force
    Remove-Item -Recurse -Force $tmpDir
}

function Write-ReleaseJson {
    param(
        [string]$Path,
        [string]$MeshcArchive,
        [string]$MeshpkgArchive,
        [string]$Version
    )

    @{
        tag_name = "v$Version"
        name = 'M034 S03 staged release'
        assets = @(
            @{ name = $MeshcArchive },
            @{ name = $MeshpkgArchive },
            @{ name = 'SHA256SUMS' }
        )
    } | ConvertTo-Json -Depth 5 | Set-Content -Path $Path
}

function Setup-PrebuiltReleaseAssets {
    param([string]$AssetDir)

    if (-not (Test-Path $AssetDir -PathType Container)) {
        Fail-Phase 'setup' "prebuilt release asset dir was missing: $AssetDir"
    }

    $meshcSource = Find-SingleFile -Dir $AssetDir -Filter "meshc-v*-$Target.zip"
    $meshpkgSource = Find-SingleFile -Dir $AssetDir -Filter "meshpkg-v*-$Target.zip"
    $checksumSource = Join-Path $AssetDir 'SHA256SUMS'
    if (-not (Test-Path $checksumSource -PathType Leaf)) {
        Fail-Phase 'setup' "missing SHA256SUMS in $AssetDir"
    }

    $script:MeshcArchive = Split-Path $meshcSource -Leaf
    $script:MeshpkgArchive = Split-Path $meshpkgSource -Leaf
    $meshcVersion = Get-VersionFromArchiveName -Prefix 'meshc' -ArchiveName $script:MeshcArchive -Target $Target -Extension 'zip'
    $meshpkgVersion = Get-VersionFromArchiveName -Prefix 'meshpkg' -ArchiveName $script:MeshpkgArchive -Target $Target -Extension 'zip'
    if ($meshcVersion -ne $meshpkgVersion) {
        Fail-Phase 'setup' "meshc ($meshcVersion) and meshpkg ($meshpkgVersion) archive versions diverged"
    }
    $script:Version = $meshcVersion

    New-Item -ItemType Directory -Path (Join-Path $GoodRoot 'api/releases'), (Join-Path $GoodRoot "snowdamiz/mesh-lang/releases/download/v$($script:Version)") -Force | Out-Null
    Copy-Item $meshcSource (Join-Path $GoodRoot "snowdamiz/mesh-lang/releases/download/v$($script:Version)/$($script:MeshcArchive)")
    Copy-Item $meshpkgSource (Join-Path $GoodRoot "snowdamiz/mesh-lang/releases/download/v$($script:Version)/$($script:MeshpkgArchive)")
    Copy-Item $checksumSource (Join-Path $GoodRoot "snowdamiz/mesh-lang/releases/download/v$($script:Version)/SHA256SUMS")
    Write-ReleaseJson -Path (Join-Path $GoodRoot 'api/releases/latest.json') -MeshcArchive $script:MeshcArchive -MeshpkgArchive $script:MeshpkgArchive -Version $script:Version
}

function Setup-LocalReleaseAssets {
    $script:Version = Get-RepoVersion
    $script:MeshcArchive = "meshc-v$($script:Version)-$Target.zip"
    $script:MeshpkgArchive = "meshpkg-v$($script:Version)-$Target.zip"

    Invoke-LoggedCommand -Phase 'tooling' -Label '03-build-tooling' -Display 'cargo build -q -p meshc -p meshpkg' -Command {
        cargo build -q -p meshc -p meshpkg
    }

    if (-not (Test-Path $MeshcExe)) { Fail-Phase 'tooling' 'meshc.exe was not built' $LastLogPath }
    if (-not (Test-Path $MeshpkgExe)) { Fail-Phase 'tooling' 'meshpkg.exe was not built' $LastLogPath }

    New-Item -ItemType Directory -Path (Join-Path $GoodRoot 'api/releases'), (Join-Path $GoodRoot "snowdamiz/mesh-lang/releases/download/v$($script:Version)") -Force | Out-Null

    New-ZipArchive -ArchivePath (Join-Path $GoodRoot "snowdamiz/mesh-lang/releases/download/v$($script:Version)/$($script:MeshcArchive)") -SourcePath $MeshcExe -EntryName 'meshc.exe'
    New-ZipArchive -ArchivePath (Join-Path $GoodRoot "snowdamiz/mesh-lang/releases/download/v$($script:Version)/$($script:MeshpkgArchive)") -SourcePath $MeshpkgExe -EntryName 'meshpkg.exe'
    $meshcSha = Get-Sha256 (Join-Path $GoodRoot "snowdamiz/mesh-lang/releases/download/v$($script:Version)/$($script:MeshcArchive)")
    $meshpkgSha = Get-Sha256 (Join-Path $GoodRoot "snowdamiz/mesh-lang/releases/download/v$($script:Version)/$($script:MeshpkgArchive)")
    Set-Content -Path (Join-Path $GoodRoot "snowdamiz/mesh-lang/releases/download/v$($script:Version)/SHA256SUMS") -Value @(
        "$meshcSha  $($script:MeshcArchive)",
        "$meshpkgSha  $($script:MeshpkgArchive)"
    )
    Write-ReleaseJson -Path (Join-Path $GoodRoot 'api/releases/latest.json') -MeshcArchive $script:MeshcArchive -MeshpkgArchive $script:MeshpkgArchive -Version $script:Version
}

function Start-LocalServer {
    param([int]$Port)

    $python = Get-PythonCommand
    $script:ServerProcess = Start-Process -FilePath $python -ArgumentList @('-m', 'http.server', $Port, '--bind', '127.0.0.1', '--directory', $ServerRoot) -RedirectStandardOutput (Join-Path $RunDir 'http-server.stdout') -RedirectStandardError (Join-Path $RunDir 'http-server.stderr') -PassThru

    $url = "http://127.0.0.1:$Port/"
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        try {
            Invoke-WebRequest -Uri $url -TimeoutSec 2 | Out-Null
            return
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }

    Fail-Phase 'server' 'local staged release server did not become ready' (Join-Path $RunDir 'http-server.stderr')
}

if ($env:M034_S03_LIB_ONLY -eq '1') {
    return
}

try {
    Remove-Item -Recurse -Force $TmpRoot -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Path $RunDir, $StageRoot, $HomeRoot, $WorkRoot -Force | Out-Null

    $script:RunDir = $RunDir
    $script:ServerRoot = $ServerRoot

    Invoke-LoggedCommand -Phase 'contract' -Label '01-ps1-diff' -Display 'Compare canonical and repo-local PowerShell installers' -Command {
        $canonicalHash = (Get-FileHash -Path $InstallScript -Algorithm SHA256).Hash
        $repoHash = (Get-FileHash -Path $RepoInstallScript -Algorithm SHA256).Hash
        if ($canonicalHash -ne $repoHash) {
            throw 'PowerShell installer copies drifted'
        }
        Write-Output "sha256=$canonicalHash"
    }
    if (-not (Test-Path $InstallScript)) { Fail-Phase 'contract' 'install.ps1 missing' }

    Invoke-LoggedCommand -Phase 'contract' -Label '02-ps1-contract' -Display 'Verify PowerShell installer covers meshpkg and staged hooks' -Command {
        Select-String -Path $InstallScript -Pattern 'snowdamiz/mesh-lang', 'meshpkg', 'MESH_INSTALL_RELEASE_API_URL', 'MESH_INSTALL_RELEASE_BASE_URL', 'MESH_INSTALL_STRICT_PROOF' | ForEach-Object { $_.Line }
    }
    foreach ($needle in @('snowdamiz/mesh-lang', 'meshpkg', 'MESH_INSTALL_RELEASE_API_URL', 'MESH_INSTALL_RELEASE_BASE_URL', 'MESH_INSTALL_STRICT_PROOF')) {
        Assert-LogContains -Phase 'contract' -Needle $needle -LogPath $LastLogPath
    }

    if ($PrebuiltReleaseDir) {
        Setup-PrebuiltReleaseAssets -AssetDir $PrebuiltReleaseDir
    } else {
        Setup-LocalReleaseAssets
    }

    Set-Content -Path (Join-Path $RunDir '00-context.log') -Value @(
        "version=$Version",
        "target=$Target",
        "prebuilt_release_dir=$($PrebuiltReleaseDir ?? 'none')",
        "verify_root=$RunDir",
        "stage_root=$StageRoot",
        "fixture_dir=$FixtureDir"
    )

    Get-ChildItem -Path $ServerRoot -File -Recurse | Sort-Object FullName | ForEach-Object { $_.FullName.Replace("$RootDir\", '') } | Set-Content -Path (Join-Path $RunDir 'staged-layout.txt')

    $serverPort = Get-FreePort
    Start-LocalServer -Port $serverPort
    $serverUrl = "http://127.0.0.1:$serverPort"
    $goodApiUrl = "$serverUrl/good/api/releases/latest.json"
    $goodBaseUrl = "$serverUrl/good/snowdamiz/mesh-lang/releases/download"

    Set-Content -Path (Join-Path $RunDir 'server-urls.log') -Value @(
        "server_url=$serverUrl",
        "good_api_url=$goodApiUrl",
        "good_base_url=$goodBaseUrl"
    )

    $env:MESH_INSTALL_RELEASE_API_URL = $goodApiUrl
    $env:MESH_INSTALL_RELEASE_BASE_URL = $goodBaseUrl
    $env:MESH_INSTALL_STRICT_PROOF = '1'
    $env:MESH_INSTALL_DOWNLOAD_TIMEOUT_SEC = '20'

    $goodHome = Join-Path $HomeRoot 'good'
    New-Item -ItemType Directory -Path $goodHome -Force | Out-Null
    $env:USERPROFILE = $goodHome

    Invoke-LoggedCommand -Phase 'install' -Label '04-install-good' -Display 'pwsh -File website/docs/public/install.ps1 -Yes' -Command {
        pwsh -NoProfile -File $script:InstallScript -Yes
    }

    $installedMeshc = Join-Path $goodHome '.mesh/bin/meshc.exe'
    $installedMeshpkg = Join-Path $goodHome '.mesh/bin/meshpkg.exe'
    $installedVersion = Join-Path $goodHome '.mesh/version'
    if (-not (Test-Path $installedMeshc)) { Fail-Phase 'install' 'installed meshc.exe was missing' $LastLogPath }
    if (-not (Test-Path $installedMeshpkg)) { Fail-Phase 'install' 'installed meshpkg.exe was missing' $LastLogPath }
    if (-not (Test-Path $installedVersion)) { Fail-Phase 'install' 'version file was not written' $LastLogPath }
    if ((Get-Content $installedVersion -Raw).Trim() -ne $Version) { Fail-Phase 'install' 'version file did not match staged version' $LastLogPath }

    Invoke-LoggedCommand -Phase 'version' -Label '05-meshc-version' -Display 'installed meshc.exe --version' -Command {
        & $installedMeshc --version
    }
    Assert-LogContains -Phase 'version' -Needle "meshc $Version" -LogPath $LastLogPath

    Invoke-LoggedCommand -Phase 'version' -Label '06-meshpkg-version' -Display 'installed meshpkg.exe --version' -Command {
        & $installedMeshpkg --version
    }
    Assert-LogContains -Phase 'version' -Needle "meshpkg $Version" -LogPath $LastLogPath

    $smokeDir = Join-Path $WorkRoot 'installer-smoke'
    New-Item -ItemType Directory -Path $smokeDir -Force | Out-Null
    Copy-Item (Join-Path $FixtureDir 'mesh.toml') (Join-Path $smokeDir 'mesh.toml') -Force
    Copy-Item (Join-Path $FixtureDir 'main.mpl') (Join-Path $smokeDir 'main.mpl') -Force
    $helloExe = Join-Path $RunDir 'installer-smoke.exe'

    Invoke-LoggedCommand -Phase 'build' -Label '07-hello-build' -Display 'installed meshc.exe build installer smoke fixture' -Command {
        & $installedMeshc build $smokeDir --output $helloExe --no-color
    }
    Invoke-LoggedCommand -Phase 'runtime' -Label '08-hello-run' -Display 'run installed hello binary' -Command {
        & $helloExe
    }
    Assert-LogContains -Phase 'runtime' -Needle 'hello' -LogPath $LastLogPath

    Write-Host 'verify-m034-s03.ps1: ok'
} finally {
    Stop-LocalServer
}
