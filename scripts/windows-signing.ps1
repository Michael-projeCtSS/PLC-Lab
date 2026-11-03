param(
    [Parameter(Mandatory = $true)][ValidateSet('Preflight', 'Sign', 'Verify', 'SignCatalog', 'VerifyCatalog')][string]$Action,
    [Parameter(Mandatory = $true)][string]$ConfigPath,
    [string]$FilePath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
# A host that launches Windows PowerShell from PowerShell 7 can inherit a module
# search path containing incompatible Core modules. Use this shell's own modules.
foreach ($module in @('Microsoft.PowerShell.Management', 'Microsoft.PowerShell.Utility', 'Microsoft.PowerShell.Security')) {
    Import-Module (Join-Path $PSHOME "Modules\$module\$module.psd1") -ErrorAction Stop
}

function Fail([string]$Prefix, [string]$Message) {
    throw "$($Prefix): $Message"
}

function Find-SignTool($Config) {
    if ($Config.PSObject.Properties.Name -contains 'signtoolPath') {
        if (!(Test-Path -LiteralPath $Config.signtoolPath -PathType Leaf)) { Fail 'SIGNING NOT CONFIGURED' 'configured Windows SDK SignTool is missing.' }
        return (Get-Item -LiteralPath $Config.signtoolPath).FullName
    }
    $sdkBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $candidates = @(Get-ChildItem -LiteralPath $sdkBin -Directory -ErrorAction SilentlyContinue | Where-Object { $_.Name -match '^10\.0\.\d+\.\d+$' } | Sort-Object { [version]$_.Name } -Descending)
    foreach ($candidate in $candidates) {
        $tool = Join-Path $candidate.FullName 'x64\signtool.exe'
        if (Test-Path -LiteralPath $tool -PathType Leaf) { return $tool }
    }
    Fail 'SIGNING NOT CONFIGURED' 'install the x64 Windows SDK SignTool or configure signtoolPath.'
}

function Invoke-SignTool([string[]]$ToolArguments, [string]$Failure) {
    # Never echo provider output: SDK diagnostics may include authentication details.
    $previousPreference = $ErrorActionPreference
    try { $ErrorActionPreference = 'Continue'; $output = & $script:SignTool @ToolArguments 2>&1 }
    finally { $ErrorActionPreference = $previousPreference }
    if ($LASTEXITCODE -ne 0) { Fail $Failure 'SignTool rejected the operation. Check the provider account, certificate, timestamp service and Windows trust diagnostics.' }
}

function Assert-Eku($Certificate, [string]$Oid, [string]$Purpose) {
    $extension = @($Certificate.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.37' })
    if ($extension.Count -ne 1 -or !(@($extension[0].EnhancedKeyUsages | ForEach-Object { $_.Value }) -contains $Oid)) {
        Fail 'SIGNATURE VERIFICATION FAILED' "certificate does not explicitly permit $Purpose."
    }
}

function Get-TrustedChain($Certificate, [string]$Purpose) {
    $chain = New-Object System.Security.Cryptography.X509Certificates.X509Chain
    $chain.ChainPolicy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::Online
    $chain.ChainPolicy.RevocationFlag = [System.Security.Cryptography.X509Certificates.X509RevocationFlag]::ExcludeRoot
    $chain.ChainPolicy.UrlRetrievalTimeout = [TimeSpan]::FromSeconds(30)
    # WinVerifyTrust (Get-AuthenticodeSignature + SignTool /pa) validates the trusted
    # timestamp and certificate validity at signing time. This additional chain pass
    # checks root identity/revocation without rejecting a normally expired signing
    # certificate whose RFC 3161 timestamp remains valid (Artifact Signing: 3 days).
    $chain.ChainPolicy.VerificationFlags = [System.Security.Cryptography.X509Certificates.X509VerificationFlags]::IgnoreNotTimeValid
    $null = $chain.ChainPolicy.ApplicationPolicy.Add((New-Object System.Security.Cryptography.Oid($Purpose)))
    if (!$chain.Build($Certificate)) { Fail 'SIGNATURE VERIFICATION FAILED' 'online certificate chain validation failed (including revocation/offline checks). Connect to the CA validation services and retry.' }
    if ($chain.ChainElements.Count -lt 2) { Fail 'SIGNATURE VERIFICATION FAILED' 'self-signed publisher certificates are refused.' }
    return $chain
}

function Read-DerElement([byte[]]$Bytes, [int]$Offset) {
    if ($Offset -lt 0 -or $Offset + 2 -gt $Bytes.Length) { Fail 'SIGNATURE VERIFICATION FAILED' 'malformed timestamp ASN.1.' }
    $tag = [int]$Bytes[$Offset]
    $cursor = $Offset + 1
    $length = [int]$Bytes[$cursor++]
    if ($length -ge 128) {
        $count = $length -band 127
        if ($count -lt 1 -or $count -gt 4 -or $cursor + $count -gt $Bytes.Length) { Fail 'SIGNATURE VERIFICATION FAILED' 'malformed timestamp DER length.' }
        $length = [long]0
        for ($i = 0; $i -lt $count; $i++) { $length = ($length * 256) + $Bytes[$cursor++] }
    }
    if ($length -lt 0 -or $cursor + $length -gt $Bytes.Length) { Fail 'SIGNATURE VERIFICATION FAILED' 'truncated timestamp DER.' }
    return @{ Tag = $tag; Offset = $cursor; Length = [int]$length; Next = [int]($cursor + $length) }
}

function Assert-Sha256Cms([byte[]]$Bytes, [switch]$Catalog) {
    Add-Type -AssemblyName System.Security
    if ($Catalog) {
        $cmsBytes = $Bytes
    } else {
    $pe = [BitConverter]::ToInt32($Bytes, 60)
    $optional = $pe + 24
    $directory = $optional + 112 + (8 * 4)
    $certificateOffset = [BitConverter]::ToInt32($Bytes, $directory)
    $certificateSize = [BitConverter]::ToInt32($Bytes, $directory + 4)
    if ($certificateOffset -le 0 -or $certificateSize -lt 8 -or ([long]$certificateOffset + $certificateSize) -gt $Bytes.Length) { Fail 'SIGNATURE VERIFICATION FAILED' 'embedded Authenticode certificate table is missing or invalid.' }
    $length = [BitConverter]::ToInt32($Bytes, $certificateOffset)
    if ($length -lt 8 -or $length -gt $certificateSize -or [BitConverter]::ToUInt16($Bytes, $certificateOffset + 6) -ne 2) { Fail 'SIGNATURE VERIFICATION FAILED' 'unsupported Authenticode certificate record.' }
    $cmsBytes = New-Object byte[] ($length - 8)
    [Array]::Copy($Bytes, $certificateOffset + 8, $cmsBytes, 0, $cmsBytes.Length)
    }
    $cms = New-Object System.Security.Cryptography.Pkcs.SignedCms
    $cms.Decode($cmsBytes)
    if ($cms.SignerInfos.Count -ne 1 -or $cms.SignerInfos[0].DigestAlgorithm.Value -ne '2.16.840.1.101.3.4.2.1') { Fail 'SIGNATURE VERIFICATION FAILED' 'the Authenticode signature must use SHA-256.' }
    if ($Catalog) {
        if ($cms.ContentInfo.ContentType.Value -ne '1.3.6.1.4.1.311.10.1') { Fail 'SIGNATURE VERIFICATION FAILED' 'signed file is not a Windows catalog.' }
    } else {
    if ($cms.ContentInfo.ContentType.Value -ne '1.3.6.1.4.1.311.2.1.4') { Fail 'SIGNATURE VERIFICATION FAILED' 'unexpected Authenticode signed content type.' }
    # Check the actual PE file digest as well as the CMS SignerInfo digest.
    $fileDer = $cms.ContentInfo.Content
    $fileSequence = Read-DerElement $fileDer 0
    $fileData = Read-DerElement $fileDer $fileSequence.Offset
    $fileDigest = Read-DerElement $fileDer $fileData.Next
    $fileAlgorithm = Read-DerElement $fileDer $fileDigest.Offset
    $fileOid = Read-DerElement $fileDer $fileAlgorithm.Offset
    if ($fileSequence.Tag -ne 48 -or $fileDigest.Tag -ne 48 -or $fileAlgorithm.Tag -ne 48 -or $fileOid.Tag -ne 6 -or $fileOid.Length -ne 9 -or [BitConverter]::ToString($fileDer, $fileOid.Offset, $fileOid.Length).Replace('-', '') -ne '608648016503040201') { Fail 'SIGNATURE VERIFICATION FAILED' 'the signed PE file digest must use SHA-256.' }
    }
    $timestamps = @($cms.SignerInfos[0].UnsignedAttributes | Where-Object { $_.Oid.Value -eq '1.3.6.1.4.1.311.3.3.1' })
    if ($timestamps.Count -ne 1 -or $timestamps[0].Values.Count -ne 1) { Fail 'SIGNATURE VERIFICATION FAILED' 'exactly one RFC 3161 timestamp is required.' }
    $timestamp = New-Object System.Security.Cryptography.Pkcs.SignedCms
    $timestamp.Decode($timestamps[0].Values[0].RawData)
    if ($timestamp.SignerInfos.Count -ne 1 -or $timestamp.SignerInfos[0].DigestAlgorithm.Value -ne '2.16.840.1.101.3.4.2.1' -or $timestamp.ContentInfo.ContentType.Value -ne '1.2.840.113549.1.9.16.1.4') { Fail 'SIGNATURE VERIFICATION FAILED' 'RFC 3161 timestamp content or SHA-256 signing digest is invalid.' }
    $der = $timestamp.ContentInfo.Content
    $sequence = Read-DerElement $der 0
    if ($sequence.Tag -ne 48) { Fail 'SIGNATURE VERIFICATION FAILED' 'invalid TSTInfo sequence.' }
    $version = Read-DerElement $der $sequence.Offset
    $policy = Read-DerElement $der $version.Next
    $imprint = Read-DerElement $der $policy.Next
    $algorithm = Read-DerElement $der $imprint.Offset
    $oid = Read-DerElement $der $algorithm.Offset
    if ($imprint.Tag -ne 48 -or $algorithm.Tag -ne 48 -or $oid.Tag -ne 6 -or $oid.Length -ne 9) { Fail 'SIGNATURE VERIFICATION FAILED' 'invalid timestamp message imprint algorithm.' }
    $hex = [BitConverter]::ToString($der, $oid.Offset, $oid.Length).Replace('-', '')
    if ($hex -ne '608648016503040201') { Fail 'SIGNATURE VERIFICATION FAILED' 'RFC 3161 timestamp message imprint must use SHA-256.' }
}

function Assert-Pe64([string]$Path) {
    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 256 -or $bytes[0] -ne 77 -or $bytes[1] -ne 90) { Fail 'SIGNATURE VERIFICATION FAILED' 'application is not a Windows PE file.' }
    $pe = [BitConverter]::ToInt32($bytes, 60)
    if ($pe -lt 64 -or $pe + 256 -gt $bytes.Length -or [BitConverter]::ToUInt32($bytes, $pe) -ne 17744 -or [BitConverter]::ToUInt16($bytes, $pe + 4) -ne 34404 -or [BitConverter]::ToUInt16($bytes, $pe + 24) -ne 523) { Fail 'SIGNATURE VERIFICATION FAILED' 'application must be a valid x64 PE32+ executable.' }
    return ,$bytes
}

function Verify-Application([string]$Path, $Config, [switch]$Catalog) {
    $hashBefore = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    $bytes = if ($Catalog) { [IO.File]::ReadAllBytes($Path) } else { Assert-Pe64 $Path }
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) { Fail 'SIGNATURE VERIFICATION FAILED' "Authenticode status is $($signature.Status); a valid publicly trusted signature is required." }
    if ($signature.SignatureType.ToString() -ne 'Authenticode') { Fail 'SIGNATURE VERIFICATION FAILED' 'an embedded Authenticode signature is required.' }
    if (!$signature.SignerCertificate -or $signature.SignerCertificate.Subject -cne $Config.publisherSubject) { Fail 'SIGNATURE VERIFICATION FAILED' 'publisher subject does not exactly match the configured verified identity.' }
    if (!$signature.TimeStamperCertificate) { Fail 'SIGNATURE VERIFICATION FAILED' 'a trusted timestamp is required.' }
    Assert-Eku $signature.SignerCertificate '1.3.6.1.5.5.7.3.3' 'code signing'
    Assert-Eku $signature.TimeStamperCertificate '1.3.6.1.5.5.7.3.8' 'time stamping'
    if ($signature.SignerCertificate.PublicKey.Oid.Value -notin @('1.2.840.113549.1.1.1', '1.2.840.10045.2.1')) { Fail 'SIGNATURE VERIFICATION FAILED' 'publisher key must be RSA or ECC supported by Smart App Control.' }
    Invoke-SignTool @('verify', '/pa', '/all', '/tw', $Path) 'SIGNATURE VERIFICATION FAILED'
    Assert-Sha256Cms $bytes -Catalog:$Catalog
    $chain = Get-TrustedChain $signature.SignerCertificate '1.3.6.1.5.5.7.3.3'
    $root = $chain.ChainElements[$chain.ChainElements.Count - 1].Certificate
    if (@($Config.trustedRootThumbprints) -notcontains $root.Thumbprint) { Fail 'SIGNATURE VERIFICATION FAILED' 'publisher chain does not terminate at a configured public CA root.' }
    $timestampChain = Get-TrustedChain $signature.TimeStamperCertificate '1.3.6.1.5.5.7.3.8'
    $hashAfter = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($hashBefore -cne $hashAfter) { Fail 'SIGNATURE VERIFICATION FAILED' 'executable changed during signature verification.' }
    return [ordered]@{
        status = 'Valid'
        sha256 = $hashAfter
        publisherSubject = $signature.SignerCertificate.Subject
        signerThumbprint = $signature.SignerCertificate.Thumbprint
        rootThumbprint = $root.Thumbprint
        timestampSubject = $signature.TimeStamperCertificate.Subject
        timestampThumbprint = $signature.TimeStamperCertificate.Thumbprint
        timestampRootThumbprint = $timestampChain.ChainElements[$timestampChain.ChainElements.Count - 1].Certificate.Thumbprint
        fileDigest = 'SHA256'
        timestampDigest = 'SHA256'
        timestampType = 'RFC3161'
        verifiedAt = [DateTime]::UtcNow.ToString('o')
    }
}

try {
    $config = Get-Content -LiteralPath $ConfigPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($config.provider -notin @('azure-artifact-signing', 'certificate-store', 'ssl-esigner') -or $config.publisherSubject -notmatch 'CN=' -or $config.publisherSubject -match 'REPLACE|YOUR|[<>]') { Fail 'SIGNING NOT CONFIGURED' 'provider and verified publisher identity are required.' }
    if (@($config.trustedRootThumbprints).Count -eq 0 -or @($config.trustedRootThumbprints | Where-Object { $_ -notmatch '^[a-fA-F0-9]{40}$' -or $_ -match '^0+$' }).Count -gt 0) { Fail 'SIGNING NOT CONFIGURED' 'pin the public CA root thumbprints using provider PKI documentation.' }
    if ($config.provider -eq 'azure-artifact-signing') {
        $regions = @('brs', 'cus', 'eus', 'jpe', 'krc', 'ncus', 'neu', 'plc', 'scus', 'swn', 'wcus', 'weu', 'wus', 'wus2', 'wus3')
        if ($config.azure.endpoint -notin @($regions | ForEach-Object { "https://$_.codesigning.azure.net" }) -or $config.azure.accountName -notmatch '^[a-zA-Z0-9-]{3,64}$' -or $config.azure.certificateProfileName -notmatch '^[a-zA-Z0-9-]{3,64}$') { Fail 'SIGNING NOT CONFIGURED' 'configure an official regional Artifact Signing endpoint and account/profile names.' }
    } elseif ($config.provider -eq 'certificate-store') {
        if ($config.certificateStore.storeLocation -notin @('CurrentUser', 'LocalMachine') -or $config.certificateStore.thumbprint -notmatch '^[a-fA-F0-9]{40}$') { Fail 'SIGNING NOT CONFIGURED' 'configure the exact certificate in CurrentUser or LocalMachine Personal store.' }
        $timestampUri = [Uri]$config.certificateStore.timestampUrl
        if (!$timestampUri.IsAbsoluteUri -or $timestampUri.Scheme -notin @('http', 'https') -or $timestampUri.UserInfo -or $timestampUri.Query -or $timestampUri.Fragment) { Fail 'SIGNING NOT CONFIGURED' 'configure the CA-documented RFC 3161 timestamp URL without credentials.' }
    }
    $script:SignTool = Find-SignTool $config
    if ($Action -in @('Verify', 'VerifyCatalog')) {
        if (!$FilePath -or !(Test-Path -LiteralPath $FilePath -PathType Leaf)) { Fail 'SIGNATURE VERIFICATION FAILED' 'executable is missing.' }
        Verify-Application (Get-Item -LiteralPath $FilePath).FullName $config -Catalog:($Action -eq 'VerifyCatalog') | ConvertTo-Json -Depth 6
        exit 0
    }
    if ($config.provider -eq 'ssl-esigner') {
        if ($Action -eq 'Preflight') {
            @{ status = 'Configured'; provider = $config.provider; publisherSubject = $config.publisherSubject; note = 'Windows SDK ready; eSigner credentials/client are checked by the JavaScript provider wrapper.' } | ConvertTo-Json
            exit 0
        }
        Fail 'SIGNING NOT CONFIGURED' 'use the protected SSL.com eSigner workflow/script for signing; this Windows tool verifies its final Authenticode and catalog signatures.'
    }
    if ($config.provider -eq 'azure-artifact-signing') {
        if (!($config.azure.PSObject.Properties.Name -contains 'dlibPath') -or !$config.azure.dlibPath -or !(Test-Path -LiteralPath $config.azure.dlibPath -PathType Leaf)) { Fail 'SIGNING NOT CONFIGURED' 'install Microsoft Artifact Signing client and set azure.dlibPath to the x64 Azure.CodeSigning.Dlib.dll.' }
        if (!(Get-Command az -ErrorAction SilentlyContinue)) { Fail 'SIGNING NOT CONFIGURED' 'install Azure CLI and authenticate using the scoped OIDC identity (or an authorized local Azure CLI login).' }
        $previousPreference = $ErrorActionPreference
        try { $ErrorActionPreference = 'Continue'; $accountOutput = & az account show --only-show-errors --output none 2>&1 }
        finally { $ErrorActionPreference = $previousPreference }
        if ($LASTEXITCODE -ne 0) { Fail 'SIGNING NOT CONFIGURED' 'Azure CLI has no authenticated session; run the protected OIDC workflow or authorized Azure CLI login.' }
    } else {
        $certificatePath = "Cert:\$($config.certificateStore.storeLocation)\My\$($config.certificateStore.thumbprint)"
        if (!(Test-Path -LiteralPath $certificatePath)) { Fail 'SIGNING NOT CONFIGURED' 'configured CA code-signing certificate is not in the Windows Personal store.' }
        $certificate = Get-Item -LiteralPath $certificatePath
        if (!$certificate.HasPrivateKey) { Fail 'SIGNING NOT CONFIGURED' 'the configured signing certificate has no available private key/HSM provider.' }
        if ($certificate.Subject -cne $config.publisherSubject) { Fail 'SIGNING NOT CONFIGURED' 'selected signing certificate publisher does not match configuration.' }
    }
    if ($Action -eq 'Preflight') {
        @{ status = 'Configured'; provider = $config.provider; publisherSubject = $config.publisherSubject; note = 'Signing authorization and final trust must still pass when signing.' } | ConvertTo-Json
        exit 0
    }
    if (!$FilePath -or !(Test-Path -LiteralPath $FilePath -PathType Leaf)) { Fail 'SIGNING FAILED' 'build executable is missing.' }
    $FilePath = (Get-Item -LiteralPath $FilePath).FullName
    if ($Action -eq 'Sign') { $null = Assert-Pe64 $FilePath }
    elseif ([IO.Path]::GetExtension($FilePath) -ine '.cat') { Fail 'SIGNING FAILED' 'catalog filename must use the .cat extension.' }
    $arguments = @('sign', '/fd', 'SHA256', '/td', 'SHA256', '/d', 'PLC Lab')
    $metadataFile = $null
    try {
        if ($config.provider -eq 'azure-artifact-signing') {
            $metadataFile = Join-Path ([IO.Path]::GetTempPath()) ('plc-lab-artifact-signing-' + [Guid]::NewGuid().ToString('N') + '.json')
            $metadata = @{
                Endpoint = $config.azure.endpoint
                CodeSigningAccountName = $config.azure.accountName
                CertificateProfileName = $config.azure.certificateProfileName
                ExcludeCredentials = @('EnvironmentCredential', 'WorkloadIdentityCredential', 'ManagedIdentityCredential', 'SharedTokenCacheCredential', 'VisualStudioCredential', 'VisualStudioCodeCredential', 'AzurePowerShellCredential', 'AzureDeveloperCliCredential', 'InteractiveBrowserCredential')
            }
            [IO.File]::WriteAllText($metadataFile, ($metadata | ConvertTo-Json), (New-Object System.Text.UTF8Encoding($false)))
            $arguments += @('/tr', 'http://timestamp.acs.microsoft.com', '/dlib', $config.azure.dlibPath, '/dmdf', $metadataFile)
        } else {
            $arguments += @('/tr', $config.certificateStore.timestampUrl, '/s', 'My', '/sha1', $config.certificateStore.thumbprint)
            if ($config.certificateStore.storeLocation -eq 'LocalMachine') { $arguments += '/sm' }
        }
        $arguments += $FilePath
        Invoke-SignTool $arguments 'SIGNING FAILED'
    } finally {
        if ($metadataFile -and (Test-Path -LiteralPath $metadataFile)) { Remove-Item -LiteralPath $metadataFile -Force }
    }
    Verify-Application $FilePath $config -Catalog:($Action -eq 'SignCatalog') | ConvertTo-Json -Depth 6
} catch {
    $message = $_.Exception.Message
    if ($message -notmatch '^(SIGNING NOT CONFIGURED|SIGNATURE VERIFICATION FAILED|SIGNING FAILED):') { $message = 'SIGNATURE VERIFICATION FAILED: Windows signing operation failed. Check the provider configuration and Windows SDK installation.' }
    [Console]::Error.WriteLine($message)
    exit 1
}
