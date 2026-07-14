<#
.SYNOPSIS
    rclonegui — build + publish helper
.DESCRIPTION
    -Check        rclone binary'lerini kontrol et
    -Download     rclone indir (GitHub Releases)
    -TauriBuild   cargo tauri build ile installer uret (.msi / .AppImage)
    -BuildLinux   WSL'de Linux binary derle
    -Release      Surum at + GitHub Release olustur
.EXAMPLE
    .\rclone-setup.ps1 -TauriBuild
    .\rclone-setup.ps1 -Release
    .\rclone-setup.ps1 -BuildLinux
#>

param(
    [switch]$Check,
    [switch]$Download,
    [switch]$TauriBuild,
    [switch]$BuildLinux,
    [switch]$Release,
    [string]$RcloneVersion = "current"
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$BinDir = Join-Path $RepoRoot "rclone-bin"

# ---- helpers ----
function Get-Version {
    $conf = Join-Path $RepoRoot "src-tauri\tauri.conf.json"
    $json = Get-Content $conf -Raw | ConvertFrom-Json
    return $json.version
}

# ---- 1 ----
function Check-Binaries {
    Write-Host "`n=== rclone Binary Kontrolu ===" -ForegroundColor Cyan
    $platforms = @{
        "windows/rclone.exe" = "Windows"
        "linux/rclone"       = "Linux"
        "osx-amd64/rclone"  = "macOS (Intel)"
        "osx-arm64/rclone"  = "macOS (ARM)"
    }
    $allOk = $true
    foreach ($relPath in $platforms.Keys) {
        $fullPath = Join-Path $BinDir $relPath
        if (Test-Path $fullPath) {
            $size = (Get-Item $fullPath).Length
            Write-Host "  [OK] $($platforms[$relPath]): $relPath ($([math]::Round($size/1KB)) KB)" -ForegroundColor Green
        } else {
            Write-Host "  [XX] $($platforms[$relPath]): $relPath — EKSIK" -ForegroundColor Red
            $allOk = $false
        }
    }
    if (-not $allOk) {
        Write-Host "  -> .\rclone-setup.ps1 -Download" -ForegroundColor Yellow
    }
}

# ---- 2 ----
function Download-Binaries {
    Write-Host "`n=== rclone Binary Indir ===" -ForegroundColor Cyan
    $targets = @(
        @{ os = "windows"; file = "rclone.exe" },
        @{ os = "linux";   file = "rclone" },
        @{ os = "osx-amd64"; file = "rclone" },
        @{ os = "osx-arm64"; file = "rclone" }
    )
    if ($RcloneVersion -eq "current") {
        Write-Host "  Son surum aliniyor..." -ForegroundColor Gray
        $release = Invoke-RestMethod "https://api.github.com/repos/rclone/rclone/releases/latest"
        $RcloneVersion = $release.tag_name
        Write-Host "  Surum: $RcloneVersion" -ForegroundColor Green
    }
    foreach ($t in $targets) {
        $outDir = Join-Path $BinDir $t.os
        $outFile = Join-Path $outDir $t.file
        New-Item $outDir -ItemType Directory -Force | Out-Null
        if (Test-Path $outFile) { Write-Host "  [OK] $($t.file) — var" -ForegroundColor Green; continue }
        $url = "https://github.com/rclone/rclone/releases/$RcloneVersion/download/rclone-$RcloneVersion-$($t.os).zip"
        $zip = "$env:TEMP\rclone-$($t.os).zip"
        Write-Host "  Indir: $($t.os)..." -ForegroundColor Yellow
        Invoke-WebRequest $url -OutFile $zip
        Expand-Archive $zip "$env:TEMP\rclone-extract" -Force
        $found = Get-ChildItem "$env:TEMP\rclone-extract" -Recurse -Filter $t.file | Select-Object -First 1
        if ($found) { Copy-Item $found.FullName $outFile; Write-Host "  [OK] $($t.file)" -ForegroundColor Green }
        Remove-Item "$env:TEMP\rclone-extract" -Recurse -Force -ErrorAction SilentlyContinue
        Remove-Item $zip -ErrorAction SilentlyContinue
    }
}

# ---- 3 ----
function Build-Tauri {
    Write-Host "`n=== Tauri Build ===" -ForegroundColor Cyan
    $version = Get-Version
    Write-Host "  Surum: $version" -ForegroundColor Gray
    Push-Location $RepoRoot
    Write-Host "  -> pnpm install" -ForegroundColor Gray
    pnpm install
    if ($LASTEXITCODE -ne 0) { Write-Host "  [XX] pnpm install hatasi" -ForegroundColor Red; Pop-Location; return }
    Write-Host "  -> cargo tauri build" -ForegroundColor Gray
    cargo tauri build
    if ($LASTEXITCODE -ne 0) { Write-Host "  [XX] Tauri build hatasi" -ForegroundColor Red; Pop-Location; return }
    Pop-Location
    # bundle ciktisini goster
    $bundles = Get-ChildItem (Join-Path $RepoRoot "src-tauri\target\release\bundle") -Recurse -Include "*.msi","*.exe","*.deb","*.AppImage" | Select-Object -First 5
    if ($bundles) {
        Write-Host "`n  Bundle ciktilari:" -ForegroundColor Green
        $bundles | ForEach-Object { Write-Host "    $($_.FullName)" -ForegroundColor White }
    }
}

# ---- 4 ----
function Test-WslAvailable {
    if (-not (Get-Command "wsl.exe" -ErrorAction SilentlyContinue)) { return $false }
    $d = wsl.exe --list --quiet 2>$null | Select-Object -First 1
    return [bool]$d
}

function Install-WslDeps {
    Write-Host "  -> Linux bagimliliklari..." -ForegroundColor Gray
    
    $oldEAP = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    wsl.exe -e dpkg -s libwebkit2gtk-4.1-dev >$null 2>$null
    $isInstalled = ($LASTEXITCODE -eq 0)
    $ErrorActionPreference = $oldEAP

    if (-not $isInstalled) {
        Write-Host "  -> Eksik paketler yukleniyor (sudo sifresi gerekebilir)..." -ForegroundColor Yellow
        wsl.exe -e sudo apt-get update
        wsl.exe -e sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libssl-dev libgtk-3-dev libayatana-appindicator3-dev
    }
}

function Build-Linux {
    Write-Host "`n=== Linux Build (WSL) ===" -ForegroundColor Cyan
    if (-not (Test-WslAvailable)) { Write-Host "  [XX] WSL yok" -ForegroundColor Red; return }

    $drive = $RepoRoot.Substring(0,1).ToLower()
    $rest = $RepoRoot.Substring(3) -replace '\\', '/'
    $wslPath = "/mnt/$drive/$rest"
    Write-Host "  Kaynak: $wslPath" -ForegroundColor Gray

    Install-WslDeps

    # Tum islemler WSL icinde - Windows karismaz
    Write-Host "`n  -> pnpm install + pnpm build (WSL)" -ForegroundColor Gray
    wsl.exe -e bash -l -c "cd '$wslPath' && pnpm install && pnpm build"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  [XX] Frontend build hatasi (WSL)" -ForegroundColor Red
        return
    }

    Write-Host "  -> cargo build --release (WSL)" -ForegroundColor Gray
    wsl.exe -e bash -l -c "cd '$wslPath/src-tauri' && cargo build --release"
    if ($LASTEXITCODE -eq 0) {
        Write-Host "`n  [OK] Linux binary:" -ForegroundColor Green
        Write-Host "     $wslPath/src-tauri/target/release/rclonegui" -ForegroundColor White

        Write-Host "`n  -> cargo tauri build (WSL) - .deb + .AppImage" -ForegroundColor Gray
        Write-Host "     Devam? (Enter = evet, N = hayir)" -ForegroundColor Yellow
        $confirm = Read-Host
        if ($confirm -ne "N" -and $confirm -ne "n") {
            wsl.exe -e bash -l -c "cd '$wslPath' && cargo tauri build"
            if ($LASTEXITCODE -eq 0) {
                Write-Host "`n  [OK] Bundle olusturuldu!" -ForegroundColor Green
                Write-Host "     $wslPath/src-tauri/target/release/bundle/" -ForegroundColor White
            } else {
                Write-Host "  [XX] Tauri bundle hatasi" -ForegroundColor Red
            }
        }
    } else {
        Write-Host "  [XX] Build hatasi" -ForegroundColor Red
    }
}

# ---- 5 ----
function Publish-Release {
    Write-Host "`n=== GitHub Release ===" -ForegroundColor Cyan
    $version = Get-Version
    $tag = "v$version"
    Write-Host "  Surum: $version -> tag: $tag" -ForegroundColor Gray

    # gh CLI kontrol
    if (-not (Get-Command "gh.exe" -ErrorAction SilentlyContinue)) {
        Write-Host "  [XX] gh CLI gerekli: winget install GitHub.cli" -ForegroundColor Red
        return
    }

    # auth kontrol
    $auth = gh auth status 2>&1 | Select-String "Logged in"
    if (-not $auth) {
        Write-Host "  [XX] gh auth login gerekli" -ForegroundColor Red
        return
    }

    # once build
    Push-Location $RepoRoot
    try {
        Write-Host "  -> pnpm install + pnpm build" -ForegroundColor Gray
        pnpm install 2>&1 | Out-Null
        pnpm build 2>&1 | Out-Null
        Write-Host "  -> cargo tauri build" -ForegroundColor Gray
        cargo tauri build 2>&1 | Out-Null
    } catch {
        Write-Host "  [XX] Build hatasi" -ForegroundColor Red
        Pop-Location; return
    }
    Pop-Location

    # tag at
    git tag $tag 2>$null
    git push origin $tag 2>$null

    # bundle'leri topla
    $bundleDir = Join-Path $RepoRoot "src-tauri\target\release\bundle"
    $assets = @()
    $assets += Get-ChildItem (Join-Path $bundleDir "msi") -Filter "*.msi" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName
    $assets += Get-ChildItem (Join-Path $bundleDir "nsis") -Filter "*.exe" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName

    if ($assets.Count -eq 0) {
        Write-Host "  [XX] Bundle bulunamadi: $bundleDir" -ForegroundColor Red
        return
    }

    Write-Host "  -> gh release create $tag" -ForegroundColor Gray
    $arg = @("release", "create", $tag, "--title", "rclonegui $version", "--notes", "rclonegui $version")
    $arg += $assets
    & "gh.exe" $arg 2>&1 | Write-Host

    if ($LASTEXITCODE -eq 0) {
        Write-Host "`n  [OK] Release olusturuldu!" -ForegroundColor Green
        Write-Host "  https://github.com/beyhano/rclonegui/releases/tag/$tag" -ForegroundColor White
    }
}

# ---- main ----
Write-Host "==============================" -ForegroundColor Cyan
Write-Host " rclonegui - Build Helper" -ForegroundColor Cyan
Write-Host "==============================" -ForegroundColor Cyan

if ($Check)      { Check-Binaries }
if ($Download)   { Download-Binaries }
if ($TauriBuild) { Build-Tauri }
if ($BuildLinux) { Build-Linux }
if ($Release)    { Publish-Release }

if (-not $Check -and -not $Download -and -not $TauriBuild -and -not $BuildLinux -and -not $Release) {
    $version = Get-Version
    Write-Host "  Surum: $version`n" -ForegroundColor Gray
    Write-Host "  .\rclone-setup.ps1 -Check          # Binary kontrol" -ForegroundColor Gray
    Write-Host "  .\rclone-setup.ps1 -Download       # rclone indir" -ForegroundColor Gray
    Write-Host "  .\rclone-setup.ps1 -TauriBuild     # Installer uret (.msi)" -ForegroundColor Gray
    Write-Host "  .\rclone-setup.ps1 -BuildLinux     # WSL ile Linux binary" -ForegroundColor Gray
    Write-Host "  .\rclone-setup.ps1 -Release        # Build + GitHub Release" -ForegroundColor Gray
}
