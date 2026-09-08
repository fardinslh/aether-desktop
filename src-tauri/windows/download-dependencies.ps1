param(
    [string]$InstallDir = $PSScriptRoot
)

$ErrorActionPreference = 'SilentlyContinue'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$baseDeps = Join-Path $env:LOCALAPPDATA "AetherDesktop\dependencies"

# 1. Check and download Aether if missing
$aetherExeInInst = Join-Path $InstallDir "aether.exe"
$aetherExeInLocal = Join-Path $baseDeps "aether\v1.9.0\aether.exe"

if (-not (Test-Path $aetherExeInInst)) {
    if (Test-Path $aetherExeInLocal) {
        Copy-Item $aetherExeInLocal -Destination $aetherExeInInst -Force
    } else {
        try {
            $rel = Invoke-RestMethod -Uri "https://api.github.com/repos/CluvexStudio/Aether/releases/latest" -Headers @{ "User-Agent" = "AetherDesktop-Installer" }
            $asset = $rel.assets | Where-Object { $_.name -like "*windows*x86_64*.zip" -or $_.name -like "*windows*amd64*.zip" } | Select-Object -First 1
            if ($asset) {
                $tempZip = Join-Path $env:TEMP "aether-installer-dl.zip"
                $tempExt = Join-Path $env:TEMP "aether-installer-ext"
                Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $tempZip -UseBasicParsing
                Expand-Archive -Path $tempZip -DestinationPath $tempExt -Force
                $found = Get-ChildItem -Path $tempExt -Filter "aether.exe" -Recurse | Select-Object -First 1
                if ($found) {
                    Copy-Item $found.FullName -Destination $aetherExeInInst -Force
                    $localTargetDir = Join-Path $baseDeps "aether\v1.9.0"
                    New-Item -ItemType Directory -Path $localTargetDir -Force | Out-Null
                    Copy-Item $found.FullName -Destination (Join-Path $localTargetDir "aether.exe") -Force
                }
                Remove-Item $tempZip -Force -ErrorAction SilentlyContinue
                Remove-Item $tempExt -Recurse -Force -ErrorAction SilentlyContinue
            }
        } catch {
            Write-Warning "Failed to download Aether dependency: $_"
        }
    }
}

# 2. Check and download sing-box if missing
$sbExeInInst = Join-Path $InstallDir "sing-box.exe"
$sbDllInInst = Join-Path $InstallDir "libcronet.dll"
$sbExeInLocal = Join-Path $baseDeps "sing-box\v1.14.0\sing-box-1.14.0-windows-amd64\sing-box.exe"
$sbDllInLocal = Join-Path $baseDeps "sing-box\v1.14.0\sing-box-1.14.0-windows-amd64\libcronet.dll"

if (-not (Test-Path $sbExeInInst)) {
    if (Test-Path $sbExeInLocal) {
        Copy-Item $sbExeInLocal -Destination $sbExeInInst -Force
        if (Test-Path $sbDllInLocal) {
            Copy-Item $sbDllInLocal -Destination $sbDllInInst -Force
        }
    } else {
        try {
            $rel = Invoke-RestMethod -Uri "https://api.github.com/repos/SagerNet/sing-box/releases/latest" -Headers @{ "User-Agent" = "AetherDesktop-Installer" }
            $asset = $rel.assets | Where-Object { $_.name -like "*windows*amd64*.zip" } | Select-Object -First 1
            if ($asset) {
                $tempZip = Join-Path $env:TEMP "singbox-installer-dl.zip"
                $tempExt = Join-Path $env:TEMP "singbox-installer-ext"
                Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $tempZip -UseBasicParsing
                Expand-Archive -Path $tempZip -DestinationPath $tempExt -Force
                $foundExe = Get-ChildItem -Path $tempExt -Filter "sing-box.exe" -Recurse | Select-Object -First 1
                $foundDll = Get-ChildItem -Path $tempExt -Filter "libcronet.dll" -Recurse | Select-Object -First 1
                if ($foundExe) {
                    Copy-Item $foundExe.FullName -Destination $sbExeInInst -Force
                    $localTargetDir = Join-Path $baseDeps "sing-box\v1.14.0\sing-box-1.14.0-windows-amd64"
                    New-Item -ItemType Directory -Path $localTargetDir -Force | Out-Null
                    Copy-Item $foundExe.FullName -Destination (Join-Path $localTargetDir "sing-box.exe") -Force
                }
                if ($foundDll) {
                    Copy-Item $foundDll.FullName -Destination $sbDllInInst -Force
                    $localTargetDir = Join-Path $baseDeps "sing-box\v1.14.0\sing-box-1.14.0-windows-amd64"
                    New-Item -ItemType Directory -Path $localTargetDir -Force | Out-Null
                    Copy-Item $foundDll.FullName -Destination (Join-Path $localTargetDir "libcronet.dll") -Force
                }
                Remove-Item $tempZip -Force -ErrorAction SilentlyContinue
                Remove-Item $tempExt -Recurse -Force -ErrorAction SilentlyContinue
            }
        } catch {
            Write-Warning "Failed to download sing-box dependency: $_"
        }
    }
}
