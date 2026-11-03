Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$os = Get-CimInstance Win32_OperatingSystem
$computer = Get-CimInstance Win32_ComputerSystem
$feature = 'UNKNOWN_REQUIRES_ELEVATION'
try { $feature = [string](Get-WindowsOptionalFeature -Online -FeatureName Containers-DisposableClientVM -ErrorAction Stop).State }
catch { if ($_.Exception.Message -notmatch 'elevat|uprawnie|administrator') { $feature = 'UNKNOWN_QUERY_FAILED' } }
$sandbox = Join-Path $env:SystemRoot 'System32\WindowsSandbox.exe'
$supported = $os.OperatingSystemSKU -in @(4, 27, 48, 49, 72, 121, 122, 125, 126, 161, 162, 164)
$available = $supported -and (Test-Path -LiteralPath $sandbox) -and $computer.HypervisorPresent -and $feature -ne 'Disabled'
[ordered]@{
    edition = $os.Caption; version = $os.Version; sku = $os.OperatingSystemSKU
    supportedEdition = $supported; hypervisorPresent = $computer.HypervisorPresent
    sandboxFeature = $feature; sandboxExecutablePresent = (Test-Path -LiteralPath $sandbox)
    sandbox = $(if ($available) { 'AVAILABLE' } else { 'NEEDS SETUP' })
    hostSecurityModified = $false
} | ConvertTo-Json
