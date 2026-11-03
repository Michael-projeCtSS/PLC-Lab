param([Parameter(Mandatory=$true)][string]$Archive, [Parameter(Mandatory=$true)][string]$Destination)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$destinationRoot = [IO.Path]::GetFullPath($Destination).TrimEnd('\')
if (Test-Path -LiteralPath $destinationRoot) { throw 'Extract to a fresh test directory.' }
$zip = [IO.Compression.ZipFile]::OpenRead($Archive)
try {
    $names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($entry in $zip.Entries) {
        $name = $entry.FullName.Replace('\','/')
        $target = [IO.Path]::GetFullPath((Join-Path $destinationRoot $name))
        if ($name.Contains(':') -or $name.Split('/') -contains '..' -or !$target.StartsWith($destinationRoot + '\',[StringComparison]::OrdinalIgnoreCase) -or !$names.Add($target) -or (($entry.ExternalAttributes -shr 16) -band 0xF000) -eq 0xA000) { throw 'Unsafe or duplicate artifact entry.' }
    }
} finally { $zip.Dispose() }
Expand-Archive -LiteralPath $Archive -DestinationPath $destinationRoot
