# Hosted-runner prerequisites only; no application-control/security policy changes.
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_OS -ne 'Windows') {
    throw 'This setup is restricted to an explicitly started GitHub Actions Windows job.'
}
if ($env:CARGO_TARGET_DIR) {
    throw 'The existing native smoke requires the default Cargo target directory.'
}
$taskVswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $taskVswhere)) { throw 'Visual Studio discovery tool is missing.' }
$taskVisualStudio = & $taskVswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($LASTEXITCODE -ne 0 -or -not $taskVisualStudio) { throw 'The runner needs the supported Visual Studio x64 C++ build tools.' }
$taskSdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Include'
if (-not (Test-Path -LiteralPath $taskSdkRoot)) { throw 'The Windows SDK is missing.' }

function Get-InstalledWebView2Version {
    $taskKeys = @(
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
        'HKCU:\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
    )
    foreach ($taskKey in $taskKeys) {
        if (Test-Path -LiteralPath $taskKey) {
            $taskVersion = (Get-ItemProperty -LiteralPath $taskKey).pv
            if ($taskVersion -and [version]$taskVersion -gt [version]'0.0.0.0') { return $taskVersion }
        }
    }
    return $null
}

$taskRuntime = Get-InstalledWebView2Version
if (-not $taskRuntime) {
    # Microsoft's documented Evergreen bootstrapper; never a test/security workaround.
    $taskInstaller = Join-Path $env:RUNNER_TEMP 'MicrosoftEdgeWebview2Setup.exe'
    Invoke-WebRequest -Uri 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $taskInstaller
    $taskSignature = Get-AuthenticodeSignature -LiteralPath $taskInstaller
    if ($taskSignature.Status -ne 'Valid' -or $taskSignature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation(?:,|$)') {
        throw 'The WebView2 bootstrapper must have a valid Microsoft signature.'
    }
    $taskInstallProcess = Start-Process -FilePath $taskInstaller -ArgumentList '/silent','/install' -Wait -PassThru -WindowStyle Hidden
    if ($taskInstallProcess.ExitCode -ne 0) { throw "WebView2 installation failed: $($taskInstallProcess.ExitCode)." }
    $taskRuntime = Get-InstalledWebView2Version
    if (-not $taskRuntime) { throw 'WebView2 installation did not produce the documented runtime registration.' }
}
Write-Output "Visual Studio C++ tools: $taskVisualStudio"
Write-Output "Windows SDK: $taskSdkRoot"
Write-Output "WebView2 Runtime: $taskRuntime"
