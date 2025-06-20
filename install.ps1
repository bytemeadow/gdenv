# gdenv installer script for Windows PowerShell
# Inspired by pkgx's installation approach

param(
    [string]$Version = "latest",
    [string]$InstallDir = ""
)

$ErrorActionPreference = "Stop"

# Colors for output
function Write-Info {
    param([string]$Message)
    Write-Host "▶ $Message" -ForegroundColor Blue
}

function Write-Success {
    param([string]$Message)
    Write-Host "✅ $Message" -ForegroundColor Green
}

function Write-Error {
    param([string]$Message)
    Write-Host "❌ $Message" -ForegroundColor Red
}

function Write-Warning {
    param([string]$Message)
    Write-Host "⚠️  $Message" -ForegroundColor Yellow
}

function Get-Architecture {
    $arch = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
    switch ($arch) {
        "AMD64" { return "x86_64" }
        "ARM64" { return "aarch64" }
        default {
            Write-Error "Unsupported architecture: $arch"
            exit 1
        }
    }
}

function Test-ShouldInstall {
    $existing = Get-Command gdenv -ErrorAction SilentlyContinue
    if (-not $existing) {
        return $true  # Not installed, should install
    }
    
    # Already installed, check if upgrade needed when installing latest
    if ($Version -eq "latest") {
        $currentVersion = & gdenv --version 2>$null | Select-String -Pattern '\d+\.\d+\.\d+' | ForEach-Object { $_.Matches[0].Value }
        if (-not $currentVersion) { $currentVersion = "0.0.0" }
        
        # Get latest version from GitHub API
        try {
            $latestRelease = Invoke-RestMethod -Uri "https://api.github.com/repos/bytemeadow/gdenv/releases/latest" -ErrorAction Stop
            $latestVersion = $latestRelease.tag_name -replace '^v', ''
            
            if ($currentVersion -ne $latestVersion) {
                Write-Info "Upgrading gdenv from $currentVersion to $latestVersion"
                return $true  # Should upgrade
            } else {
                Write-Info "gdenv $currentVersion is already up to date at $($existing.Source)"
                return $false
            }
        } catch {
            Write-Info "gdenv $currentVersion is already up to date at $($existing.Source)"
            return $false
        }
    } else {
        # Installing specific version, allow reinstall
        $currentVersion = & gdenv --version 2>$null | Select-String -Pattern '\d+\.\d+\.\d+' | ForEach-Object { $_.Matches[0].Value }
        if (-not $currentVersion) { $currentVersion = "unknown" }
        Write-Info "Reinstalling gdenv $Version (current: $currentVersion)"
        return $true
    }
}

function Get-InstallDirectory {
    if ($InstallDir) {
        return $InstallDir
    }

    # Try common locations
    $candidates = @(
        "$env:LOCALAPPDATA\Programs\gdenv",
        "$env:USERPROFILE\.local\bin",
        "$env:USERPROFILE\bin"
    )

    foreach ($dir in $candidates) {
        if (Test-Path $dir -PathType Container) {
            return $dir
        }
    }

    # Default to user programs directory
    $defaultDir = "$env:LOCALAPPDATA\Programs\gdenv"
    New-Item -ItemType Directory -Path $defaultDir -Force | Out-Null
    return $defaultDir
}

function Download-Gdenv {
    param([string]$InstallDirectory)

    $arch = Get-Architecture
    $baseUrl = "https://github.com/bytemeadow/gdenv/releases"

    if ($Version -eq "latest") {
        $downloadUrl = "$baseUrl/latest/download/gdenv-windows-$arch.exe"
    } else {
        $downloadUrl = "$baseUrl/download/v$Version/gdenv-windows-$arch.exe"
    }

    Write-Info "Downloading gdenv from $downloadUrl"

    $tempFile = [System.IO.Path]::GetTempFileName() + ".exe"

    try {
        $progressPreference = 'SilentlyContinue'
        Invoke-WebRequest -Uri $downloadUrl -OutFile $tempFile -UseBasicParsing
        $progressPreference = 'Continue'

        if (-not (Test-Path $tempFile)) {
            throw "Download failed"
        }

        Write-Success "Downloaded successfully"
        return $tempFile
    } catch {
        Write-Error "Failed to download gdenv: $($_.Exception.Message)"
        exit 1
    }
}

function Install-Binary {
    param([string]$TempFile, [string]$InstallDirectory)

    Write-Info "Installing gdenv..."

    $targetPath = Join-Path $InstallDirectory "gdenv.exe"

    try {
        Copy-Item $TempFile $targetPath -Force
        Remove-Item $TempFile -Force
        Write-Success "gdenv installed successfully!"
        return $targetPath
    } catch {
        Write-Error "Failed to install gdenv: $($_.Exception.Message)"
        exit 1
    }
}

function Update-Path {
    param([string]$InstallDirectory)

    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")

    if ($currentPath -like "*$InstallDirectory*") {
        Write-Info "✓ $InstallDirectory is already in your PATH"
        return
    }

    Write-Info "Adding $InstallDirectory to your PATH"
    try {
        $newPath = "$InstallDirectory;$currentPath"
        [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        Write-Success "Added $InstallDirectory to your PATH"
        Write-Info "Restart your terminal or run 'refreshenv' to use gdenv"
    } catch {
        Write-Warning "Failed to update PATH automatically"
        Write-Info "Please manually add $InstallDirectory to your PATH"
    }
}

function Main {
    Write-Host @"
    ┌──────────────────────────────┐
    │  gdenv - Godot Environment   │
    │        Manager               │
    │  https://gdenv.bytemeadow.com │
    └──────────────────────────────┘
"@ -ForegroundColor Blue

    if (Test-ShouldInstall) {
        $installDir = Get-InstallDirectory
        Write-Info "Installing to: $installDir"

        $tempFile = Download-Gdenv -InstallDirectory $installDir
        $binaryPath = Install-Binary -TempFile $tempFile -InstallDirectory $installDir
        Update-Path -InstallDirectory $installDir

        Write-Host ""
        Write-Success "Installation complete! 🎉"
        Write-Info "Run 'gdenv --help' to get started"
        Write-Info "Install a Godot version with: gdenv install 4.2.1"
    }
}

# Run main function
Main
