param(
    [Parameter(Mandatory = $true)][ValidateSet('Create', 'VerifyMembers')][string]$Action,
    [Parameter(Mandatory = $true)][string]$InventoryPath,
    [Parameter(Mandatory = $true)][string]$CatalogPath,
    [string]$ToolPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
foreach ($module in @('Microsoft.PowerShell.Management', 'Microsoft.PowerShell.Utility', 'Microsoft.PowerShell.Security')) {
    Import-Module (Join-Path $PSHOME "Modules\$module\$module.psd1") -ErrorAction Stop
}

function Fail([string]$Prefix, [string]$Message) { throw "$($Prefix): $Message" }
function Find-SdkTool([string]$Name) {
    if ($ToolPath) {
        if (!(Test-Path -LiteralPath $ToolPath -PathType Leaf)) { Fail 'SIGNING NOT CONFIGURED' "configured Windows SDK $Name is missing." }
        return (Get-Item -LiteralPath $ToolPath).FullName
    }
    $sdk = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    foreach ($version in @(Get-ChildItem -LiteralPath $sdk -Directory -ErrorAction SilentlyContinue | Where-Object { $_.Name -match '^10\.0\.\d+\.\d+$' } | Sort-Object { [version]$_.Name } -Descending)) {
        $candidate = Join-Path $version.FullName "x64\$Name"
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    Fail 'SIGNING NOT CONFIGURED' "install the x64 Windows SDK including $Name; see docs/WINDOWS_CODE_SIGNING.md."
}

$cdf = $null
$temporary = $null
try {
    $members = Get-Content -LiteralPath $InventoryPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($members -isnot [Array]) { $members = @($members) }
    if ($members.Count -lt 2) { Fail 'SIGNATURE VERIFICATION FAILED' 'the catalog must bind product.json and the application.' }
    foreach ($member in $members) {
        if ($member.file -match '[\r\n\x00"]' -or !(Test-Path -LiteralPath $member.file -PathType Leaf) -or (Get-FileHash -LiteralPath $member.file -Algorithm SHA256).Hash.ToLowerInvariant() -cne $member.sha256) { Fail 'SIGNATURE VERIFICATION FAILED' 'catalog input changed or contains an unsafe path.' }
    }
    if ($Action -eq 'Create') {
        $tool = Find-SdkTool 'makecat.exe'
        if (Test-Path -LiteralPath $CatalogPath) { Fail 'SIGNATURE VERIFICATION FAILED' 'product.cat already exists; prepare a fresh distribution before generating its signed catalog.' }
        if ($CatalogPath -match '[\r\n\x00"]' -or [IO.Path]::GetFileName($CatalogPath) -cne 'product.cat') { Fail 'SIGNATURE VERIFICATION FAILED' 'invalid manifest catalog output path.' }
        $temporaryRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
        $temporary = [IO.Path]::GetFullPath((Join-Path $temporaryRoot ('plc-lab-manifest-' + [Guid]::NewGuid().ToString('N'))))
        if (!$temporary.StartsWith($temporaryRoot + '\', [StringComparison]::OrdinalIgnoreCase)) { Fail 'SIGNATURE VERIFICATION FAILED' 'unsafe catalog temporary directory.' }
        $null = New-Item -ItemType Directory -Path $temporary
        $cdf = Join-Path $temporary 'inventory.cdf'
        $lines = @('[CatalogHeader]', 'Name=product.cat', 'ResultDir=.', 'CatalogVersion=2', 'HashAlgorithms=SHA256', 'PageHashes=false', 'EncodingType=0x00010001', '[CatalogFiles]')
        for ($i = 0; $i -lt $members.Count; $i++) {
            $extension = [IO.Path]::GetExtension($members[$i].file)
            if ($extension -and $extension -notmatch '^\.[a-zA-Z0-9_-]+$') { Fail 'SIGNATURE VERIFICATION FAILED' 'catalog member filename extensions must be ASCII.' }
            $alias = 'member-' + $i + $extension
            $aliasPath = Join-Path $temporary $alias
            [IO.File]::Copy($members[$i].file, $aliasPath, $false)
            if ((Get-FileHash -LiteralPath $aliasPath -Algorithm SHA256).Hash.ToLowerInvariant() -cne $members[$i].sha256) { Fail 'SIGNATURE VERIFICATION FAILED' 'catalog copy differs from its final distribution member.' }
            $lines += ('<HASH>File' + $i + '=' + $alias)
        }
        # MakeCat's legacy CDF parser rejects UTF-16. Controlled ASCII aliases preserve
        # each file extension/SIP and exact bytes, avoiding ANSI-path data loss for
        # Polish/Unicode/spaces/apostrophe paths. <HASH> tags bind content, not aliases.
        [IO.File]::WriteAllText($cdf, (($lines -join "`r`n") + "`r`n"), [Text.Encoding]::ASCII)
        Push-Location -LiteralPath $temporary
        try {
            $previousPreference = $ErrorActionPreference
            try { $ErrorActionPreference = 'Continue'; $output = & $tool '-r' 'inventory.cdf' 2>&1 }
            finally { $ErrorActionPreference = $previousPreference }
            if ($LASTEXITCODE -ne 0 -or !(Test-Path -LiteralPath (Join-Path $temporary 'product.cat') -PathType Leaf)) { Fail 'SIGNATURE VERIFICATION FAILED' 'MakeCat could not build the SHA-256 inventory; verify Windows SDK and readable final distribution files.' }
        } finally { Pop-Location }
        [IO.File]::Move((Join-Path $temporary 'product.cat'), $CatalogPath)
    } else {
        $tool = Find-SdkTool 'signtool.exe'
        if (!(Test-Path -LiteralPath $CatalogPath -PathType Leaf)) { Fail 'SIGNATURE VERIFICATION FAILED' 'signed product.cat is missing.' }
        foreach ($member in $members) {
            $previousPreference = $ErrorActionPreference
            try { $ErrorActionPreference = 'Continue'; $output = & $tool 'verify' '/pa' '/tw' '/hash' 'SHA256' '/c' $CatalogPath $member.file 2>&1 }
            finally { $ErrorActionPreference = $previousPreference }
            if ($LASTEXITCODE -ne 0) { Fail 'SIGNATURE VERIFICATION FAILED' "signed manifest catalog does not authenticate member: $($member.name)" }
        }
    }
    foreach ($member in $members) {
        if ((Get-FileHash -LiteralPath $member.file -Algorithm SHA256).Hash.ToLowerInvariant() -cne $member.sha256) { Fail 'SIGNATURE VERIFICATION FAILED' 'distribution changed during catalog processing.' }
    }
    @{ status = 'Complete'; catalogPath = $CatalogPath; members = $members.Count; hashAlgorithm = 'SHA256' } | ConvertTo-Json
} catch {
    $message = $_.Exception.Message
    if ($message -notmatch '^(SIGNING NOT CONFIGURED|SIGNATURE VERIFICATION FAILED):') { $message = 'SIGNATURE VERIFICATION FAILED: Windows manifest catalog operation failed; check the final distribution and Windows SDK.' }
    [Console]::Error.WriteLine($message)
    exit 1
} finally {
    if ($temporary -and (Test-Path -LiteralPath $temporary)) {
        # The absolute GUID directory was checked beneath the OS temporary root before creation.
        Remove-Item -LiteralPath $temporary -Recurse -Force
    }
}
