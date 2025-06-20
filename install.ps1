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

function Test-ExistingInstallation {
    $existing = Get-Command gdenv -ErrorAction SilentlyContinue
    if ($existing) {
        $currentVersion = & gdenv --version 2>$null | Select-String -Pattern '\d+\.\d+\.\d+' | ForEach-Object { $_.Matches[0].Value }
        if (-not $currentVersion) { $currentVersion = "unknown" }
        
        Write-Warning "gdenv $currentVersion is already installed at $($existing.Source)"
        $response = Read-Host "Do you want to reinstall? [y/N]"
        if ($response -notmatch '^[yY]([eE][sS])?$') {
            Write-Info "Installation cancelled"
            exit 0
        }
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
    $baseUrl = "https://github.com/dcvz/gdenv/releases"
    
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
    
    Write-Warning "$InstallDirectory is not in your PATH"
    
    $response = Read-Host "Add $InstallDirectory to your PATH? [Y/n]"
    if ($response -match '^[nN]([oO])?$') {
        Write-Info "You can manually add $InstallDirectory to your PATH later"
        return
    }
    
    try {
        $newPath = "$InstallDirectory;$currentPath"
        [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        Write-Success "Added $InstallDirectory to your PATH"
        Write-Info "Restart your terminal or run 'refreshenv' to use gdenv"
    } catch {
        Write-Error "Failed to update PATH: $($_.Exception.Message)"
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
    
    Test-ExistingInstallation
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

# Run main function
Main