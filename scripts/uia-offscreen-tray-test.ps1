# 实验二：任务栏移到屏幕外显示（可见但不可见），XAML 是否照常渲染？
# 挪回原位 Invoke chevron 后立刻隐藏，弹层是否仍能出现并留在正确位置？
# 目标：任务栏在屏时间 1.2s -> 几十 ms。
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;
public struct RECT2 { public int L, T, R, B; }
public class Win2 {
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int w, int hh, uint flags);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT2 r);
}
"@

function Find-Flyout {
    [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
        [System.Windows.Automation.TreeScope]::Children,
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ClassNameProperty,
            "TopLevelWindowForOverflowXamlIsland")))
}

$h = [Win2]::FindWindow("Shell_TrayWnd", $null)
$r0 = New-Object RECT2
[Win2]::GetWindowRect($h, [ref]$r0) | Out-Null
Write-Output ("taskbar rect: {0},{1} - {2},{3}" -f $r0.L, $r0.T, $r0.R, $r0.B)

# ① 隐藏（模拟桌面接管态）
[Win2]::ShowWindow($h, 0) | Out-Null
Start-Sleep -Milliseconds 500

# ② 屏幕外显示：先挪出屏幕，再显示
[Win2]::SetWindowPos($h, [IntPtr]::Zero, 0, -300, 0, 0, 0x0001 -bor 0x0002 -bor 0x0010) | Out-Null  # NOSIZE|NOZORDER|NOACTIVATE
[Win2]::ShowWindow($h, 8) | Out-Null   # SW_SHOWNA
Start-Sleep -Milliseconds 800
$r1 = New-Object RECT2
[Win2]::GetWindowRect($h, [ref]$r1) | Out-Null
Write-Output ("offscreen shown, rect: {0},{1} visible: {2}" -f $r1.L, $r1.T, [Win2]::IsWindowVisible($h))

# ③ 屏幕外状态下 UIA 枚举
$chevron = $null
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
    $btns = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    $sw.Stop()
    Write-Output ("offscreen taskbar UIA buttons: {0} ({1}ms)" -f $btns.Count, $sw.ElapsedMilliseconds)
    foreach ($b in $btns) {
        try {
            $n = $b.Current.Name
            if ($n -and $n.Contains([string][char]0x663E + [char]0x793A + [char]0x9690 + [char]0x85CF + [char]0x7684 + [char]0x56FE + [char]0x6807)) {
                $chevron = $b
            }
        } catch {}
    }
} catch {
    Write-Output ("UIA FAILED offscreen: {0}" -f $_.Exception.Message)
}
Write-Output ("chevron found offscreen: {0}" -f ($null -ne $chevron))

if ($chevron) {
    # ④ 挪回原位 -> Invoke -> 立刻隐藏（0ms 延迟）
    [Win2]::SetWindowPos($h, [IntPtr]::Zero, $r0.L, $r0.T, 0, 0, 0x0001 -bor 0x0002 -bor 0x0010) | Out-Null
    $inv = $chevron.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $inv.Invoke()
    $invokeMs = $sw.ElapsedMilliseconds
    [Win2]::ShowWindow($h, 0) | Out-Null
    Write-Output ("invoked + hid taskbar immediately")

    Start-Sleep -Milliseconds 1200
    $fly = Find-Flyout
    if ($fly) {
        Write-Output ("FLYOUT OK (0ms delay): '{0}' rect={1}" -f $fly.Current.Name, $fly.Current.BoundingRectangle)
        try {
            $w = $fly.GetCurrentPattern([System.Windows.Automation.WindowPattern]::Pattern)
            $w.Close()
            Write-Output "flyout closed"
        } catch { Write-Output "flyout close not supported (dismiss manually)" }
    } else {
        Write-Output "FLYOUT MISSING at 0ms delay"
    }
}

# ⑤ 恢复任务栏原位可见（当前非接管态）
[Win2]::SetWindowPos($h, [IntPtr]::Zero, $r0.L, $r0.T, ($r0.R - $r0.L), ($r0.B - $r0.T), 0x0002 -bor 0x0010) | Out-Null
[Win2]::ShowWindow($h, 5) | Out-Null
Start-Sleep -Milliseconds 300
Write-Output ("restored: visible={0} rect={1},{2}" -f [Win2]::IsWindowVisible($h), $r0.L, $r0.T)
