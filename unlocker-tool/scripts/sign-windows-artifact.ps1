# Authenticode-sign a single Windows artifact (NSIS .exe or MSI).
# Invoked by Tauri's `bundle.windows.signCommand` so signing happens inside
# the bundle step — before the updater .sig is computed against the file.
#
# Tauri replaces %1 with the artifact path.

param([Parameter(Mandatory=$true)][string]$Path)

$ErrorActionPreference = "Stop"

$sdkBase = "C:\Program Files (x86)\Windows Kits\10\bin"
$sdkPath = Get-ChildItem $sdkBase -Directory -Filter "10.*" -ErrorAction SilentlyContinue |
    Sort-Object Name -Descending | Select-Object -First 1
if (-not $sdkPath) { Write-Error "Windows SDK not found at $sdkBase"; exit 1 }
$signtool = Join-Path $sdkPath.FullName "x64\signtool.exe"
if (-not (Test-Path $signtool)) { Write-Error "signtool.exe not found at $signtool"; exit 1 }

$cert = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert -ErrorAction SilentlyContinue |
    Where-Object { $_.Subject -like "*SoFriendly*" }
if (-not $cert) { Write-Error "Code signing cert not found — plug in Sectigo USB token"; exit 1 }

& $signtool sign /sha1 $cert.Thumbprint /fd SHA256 /tr http://timestamp.sectigo.com /td SHA256 $Path
exit $LASTEXITCODE
