# Window automation helper for tests/e2e/launcher-smoke.mjs: drives the real Launcher.exe
# window through standard Win32 window messages (the controls are ordinary STATIC, BUTTON
# and EDIT windows). Clicks and close requests are posted, so a modal dialog opened by a
# click never blocks this helper. One action per call; the result is written to standard
# output (JSON or "ok"). Any failure is a non-zero exit with a reason.
param(
  [Parameter(Mandatory = $true)][int]$ProcessId,
  [Parameter(Mandatory = $true)]
  [ValidateSet('read', 'invoke', 'close', 'screenshot', 'diagnostics', 'open-file')]
  [string]$Action,
  [string]$Name = '',
  [string]$Value = '',
  [int]$Seconds = 20
)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class LauncherWindows {
    public delegate bool EnumProc(IntPtr hwnd, IntPtr lParam);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
    [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr parent, EnumProc f, IntPtr l);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr dialog, int id);
    [DllImport("user32.dll")] static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern IntPtr SendMessage(IntPtr h, uint msg, IntPtr w, StringBuilder l);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern IntPtr SendMessage(IntPtr h, uint msg, IntPtr w, string l);
    [DllImport("user32.dll")] static extern IntPtr SendMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }

    const uint WM_SETTEXT = 0x000C, WM_GETTEXT = 0x000D, WM_GETTEXTLENGTH = 0x000E;
    const uint WM_CLOSE = 0x0010, BM_CLICK = 0x00F5;

    public static string ClassOf(IntPtr h) { var s = new StringBuilder(256); GetClassName(h, s, 256); return s.ToString(); }
    // WM_GETTEXT is marshalled across processes, unlike GetWindowText for edit controls.
    public static string TextOf(IntPtr h) {
        int n = SendMessage(h, WM_GETTEXTLENGTH, IntPtr.Zero, IntPtr.Zero).ToInt32();
        var s = new StringBuilder(n + 1);
        SendMessage(h, WM_GETTEXT, new IntPtr(n + 1), s);
        return s.ToString();
    }
    public static void SetText(IntPtr h, string text) { SendMessage(h, WM_SETTEXT, IntPtr.Zero, text); }
    public static bool Click(IntPtr h) { return PostMessage(h, BM_CLICK, IntPtr.Zero, IntPtr.Zero); }
    public static bool Close(IntPtr h) { return PostMessage(h, WM_CLOSE, IntPtr.Zero, IntPtr.Zero); }

    public static IntPtr TopLevel(int processId, string title) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((h, l) => {
            uint pid;
            GetWindowThreadProcessId(h, out pid);
            if (pid == processId && IsWindowVisible(h) && TextOf(h) == title) { found = h; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
    public static List<IntPtr> Children(IntPtr parent) {
        var all = new List<IntPtr>();
        EnumChildWindows(parent, (h, l) => { all.Add(h); return true; }, IntPtr.Zero);
        return all;
    }
}
'@

function Find-Window([string]$title) {
  $deadline = (Get-Date).AddSeconds($Seconds)
  while ((Get-Date) -lt $deadline) {
    $window = [LauncherWindows]::TopLevel($ProcessId, $title)
    if ($window -ne [IntPtr]::Zero) { return $window }
    Start-Sleep -Milliseconds 200
  }
  throw "No window '$title' of process $ProcessId appeared within $Seconds s."
}

function Find-Child($window, [string]$class, [string]$text) {
  foreach ($child in [LauncherWindows]::Children($window)) {
    if ([LauncherWindows]::ClassOf($child) -eq $class -and ($text -eq '' -or [LauncherWindows]::TextOf($child) -eq $text)) {
      return $child
    }
  }
  throw "No $class '$text' in window '$([LauncherWindows]::TextOf($window))'."
}

function Click-Button($window, [string]$label) {
  $button = Find-Child $window 'Button' $label
  if (-not [LauncherWindows]::IsWindowEnabled($button)) { throw "Button '$label' is disabled." }
  if (-not [LauncherWindows]::Click($button)) { throw "Button '$label' could not be clicked." }
}

$launcher = 'PLC Lab'
switch ($Action) {
  'read' {
    $window = Find-Window $launcher
    $texts = @()
    $buttons = @()
    foreach ($child in [LauncherWindows]::Children($window)) {
      $class = [LauncherWindows]::ClassOf($child)
      $text = [LauncherWindows]::TextOf($child)
      if ($class -eq 'Static') { $texts += $text }
      elseif ($class -eq 'Button') {
        $buttons += [pscustomobject]@{ name = $text; enabled = [LauncherWindows]::IsWindowEnabled($child) }
      }
    }
    [pscustomobject]@{ title = [LauncherWindows]::TextOf($window); texts = $texts; buttons = $buttons } |
      ConvertTo-Json -Depth 4 -Compress
  }
  'invoke' {
    Click-Button (Find-Window $launcher) $Name
    'ok'
  }
  'close' {
    if (-not [LauncherWindows]::Close((Find-Window $launcher))) { throw 'The close request was not delivered.' }
    'ok'
  }
  'screenshot' {
    $window = Find-Window $(if ($Name) { $Name } else { $launcher })
    [void][LauncherWindows]::SetForegroundWindow($window)
    Start-Sleep -Milliseconds 400
    $r = New-Object LauncherWindows+RECT
    if (-not [LauncherWindows]::GetWindowRect($window, [ref]$r)) { throw 'The window position is unknown.' }
    $bitmap = [System.Drawing.Bitmap]::new($r.Right - $r.Left, $r.Bottom - $r.Top)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($r.Left, $r.Top, 0, 0, $bitmap.Size)
    $bitmap.Save($Value, [System.Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
    'ok'
  }
  'diagnostics' {
    $dialog = Find-Window 'PLC Lab diagnostics'
    $text = [LauncherWindows]::TextOf((Find-Child $dialog 'Edit' ''))
    [System.Windows.Forms.Clipboard]::SetText('launcher-smoke: clipboard not yet written')
    Click-Button $dialog 'Copy Diagnostics'
    Start-Sleep -Milliseconds 500
    $clipboard = [System.Windows.Forms.Clipboard]::GetText()
    Click-Button $dialog 'Close'
    [pscustomobject]@{ text = $text; clipboard = $clipboard } | ConvertTo-Json -Compress
  }
  'open-file' {
    # The standard Windows file dialog: the "File name" box (control 1148) and Open (IDOK).
    $dialog = Find-Window 'Open a PLC Lab project'
    $deadline = (Get-Date).AddSeconds($Seconds)
    $box = [IntPtr]::Zero
    while ($box -eq [IntPtr]::Zero -and (Get-Date) -lt $deadline) {
      $box = [LauncherWindows]::GetDlgItem($dialog, 1148)
      if ($box -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 200 }
    }
    if ($box -eq [IntPtr]::Zero) { throw 'The file dialog has no file name box.' }
    $edit = if ([LauncherWindows]::ClassOf($box) -eq 'Edit') { $box } else { Find-Child $box 'Edit' '' }
    [LauncherWindows]::SetText($edit, $Value)
    Start-Sleep -Milliseconds 300
    $open = [LauncherWindows]::GetDlgItem($dialog, 1)
    if ($open -eq [IntPtr]::Zero -or -not [LauncherWindows]::Click($open)) { throw 'The Open button could not be clicked.' }
    'ok'
  }
}
