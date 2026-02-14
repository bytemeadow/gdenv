param(
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$ErrorActionPreference = "Stop"

if (-not (Get-Command godot -ErrorAction SilentlyContinue)) {
    throw "godot executable was not found in PATH"
}

function Get-NormalizedReleaseTag {
    param([string]$RawVersion)

    if ($RawVersion.Contains("-")) {
        return $RawVersion
    }

    return "$RawVersion-stable"
}

$versionLine = (& godot --version | Select-Object -First 1)
$runtimeVersion = ($versionLine -split "\s+")[0]

$templateFolder = ""
$releaseTag = ""

if ($runtimeVersion -match '^(\d+\.\d+(?:\.\d+)?\.[A-Za-z0-9]+)') {
    $templateFolder = $Matches[1]
}

if ($templateFolder -match '^(\d+\.\d+(?:\.\d+)?)\.([A-Za-z]+)(\d*)$') {
    $releaseTag = "$($Matches[1])-$($Matches[2])$($Matches[3])"
}

if ([string]::IsNullOrEmpty($releaseTag)) {
    $releaseTag = Get-NormalizedReleaseTag -RawVersion $Version
}

if ([string]::IsNullOrEmpty($templateFolder)) {
    $templateFolder = [regex]::Replace($releaseTag, '-', '.', 1)
}

$templatesRoot = Join-Path $env:APPDATA "Godot\export_templates"
$destination = Join-Path $templatesRoot $templateFolder
$versionFile = Join-Path $destination "version.txt"

if (Test-Path $versionFile) {
    Write-Host "Export templates already installed at $destination"
    exit 0
}

New-Item -ItemType Directory -Path $destination -Force | Out-Null

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("gdenv-templates-" + [guid]::NewGuid().ToString("N"))
$archivePath = Join-Path $tempDir "export_templates.tpz"
$extractDir = Join-Path $tempDir "extracted"
New-Item -ItemType Directory -Path $extractDir -Force | Out-Null

$downloadUrl = "https://github.com/godotengine/godot-builds/releases/download/$releaseTag/Godot_v$releaseTag" + "_export_templates.tpz"
Write-Host "Downloading export templates from $downloadUrl"
Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing

Expand-Archive -Path $archivePath -DestinationPath $extractDir -Force

$sourceDir = $extractDir
$templatesSubdir = Join-Path $extractDir "templates"
if (Test-Path $templatesSubdir) {
    $sourceDir = $templatesSubdir
}

Copy-Item -Path (Join-Path $sourceDir "*") -Destination $destination -Recurse -Force

Remove-Item $tempDir -Recurse -Force
Write-Host "Installed export templates to $destination"
