# 任务栏可见性断言（真机测试用）。用法: powershell -File scripts/taskbar-baseline.ps1
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class TB {
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr FindWindow(string cls, string title);
}
"@
$h = [TB]::FindWindow('Shell_TrayWnd', $null)
if ($h -eq [IntPtr]::Zero) { Write-Output 'TRAYWND: not-found'; exit 2 }
$vis = [TB]::IsWindowVisible($h)
Write-Output ("TRAYWND visible: {0}" -f $vis)
if ($vis) { exit 0 } else { exit 1 }
