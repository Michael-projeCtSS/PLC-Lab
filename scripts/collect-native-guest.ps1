param([string]$Product='C:\PLC-Lab-Test\Input\product', [string]$Output='C:\PLC-Lab-Test\Results', [ValidateSet('PASS','BLOCKED','NOT TESTED')][string]$ManualResult='NOT TESTED')
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$computer = Get-CimInstance Win32_ComputerSystem
if ($env:USERNAME -ne 'WDAGUtilityAccount' -and $computer.Model -notmatch 'Virtual Machine|VirtualBox|VMware|KVM|QEMU') { throw 'Run this evidence collector only inside the selected Sandbox or virtual machine, not on the host.' }
$productRoot = [IO.Path]::GetFullPath($Product)
$outputRoot = [IO.Path]::GetFullPath($Output)
$manifest = Get-Content -LiteralPath (Join-Path $productRoot 'resources\product.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if ($manifest.releaseKind -ne 'UNSIGNED_DEVELOPMENT_BUILD') { throw 'An explicitly labeled unsigned development distribution is required.' }
$records = @(@{name='PLC Lab.exe';sha256=$manifest.application.sha256}) + @($manifest.dependencies) + @($manifest.files | ForEach-Object { @{name=('resources/' + $_.name);sha256=$_.sha256} })
if ($manifest.fallback.name -ne 'RUN_PLC_LAB.bat') { throw 'Missing fallback inventory.' }
$records += @{name=$manifest.fallback.name;sha256=$manifest.fallback.sha256}
foreach ($record in $records) {
    $target = [IO.Path]::GetFullPath((Join-Path $productRoot $record.name))
    if (!$target.StartsWith($productRoot.TrimEnd('\') + '\',[StringComparison]::OrdinalIgnoreCase) -or $record.name -match '(^|[\\/])\.\.([\\/]|$)|:') { throw 'Unsafe distribution inventory path.' }
    if ((Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash -ine $record.sha256) { throw "Distribution hash mismatch: $($record.name)" }
}
$null = New-Item -ItemType Directory -Path $outputRoot -Force
$events = @(Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-CodeIntegrity/Operational';StartTime=(Get-Date).AddHours(-2);Id=3077,3033} -ErrorAction SilentlyContinue | Where-Object {$_.Message -match 'PLC Lab.exe'} | Select-Object TimeCreated,Id,Message)
$events | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath (Join-Path $outputRoot 'code-integrity.json') -Encoding UTF8
$processes = @(Get-Process -Name 'PLC Lab' -ErrorAction SilentlyContinue | Select-Object Id,ProcessName,MainWindowTitle)
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bounds = [Windows.Forms.SystemInformation]::VirtualScreen
$bitmap = New-Object Drawing.Bitmap($bounds.Width,$bounds.Height)
$graphics = [Drawing.Graphics]::FromImage($bitmap)
try { $graphics.CopyFromScreen($bounds.Left,$bounds.Top,0,0,$bounds.Size); $bitmap.Save((Join-Path $outputRoot 'guest-desktop.png'),[Drawing.Imaging.ImageFormat]::Png) }
finally { $graphics.Dispose(); $bitmap.Dispose() }
[ordered]@{result=$ManualResult;integrity='PASS';classification='UNSIGNED NATIVE DEVELOPMENT BUILD';sourceCommit=$manifest.sourceCommit;guest=$computer.Model;processes=$processes;codeIntegrityEvents=$events.Count;simulationTest='Manual result only; no automated grading or simulation PASS is inferred';securityModified=$false} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $outputRoot 'result.json') -Encoding UTF8
Write-Output "Guest evidence saved to $outputRoot. Close the guest after retaining the results."
