# 实验三：1) 关掉遗留弹层 2) 已显示的任务栏被挪出屏幕后是否会被弹回停靠位？
# 若不弹回：显示 -> 立刻挪出屏 -> XAML 照常渲染 -> 挪回 -> Invoke -> 立即隐藏
# 全程在屏时间 ~30-60ms，肉眼不可见。
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;
public struct RECT3 { public int L, T, R, B; }
public class Win3 {
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int w, int hh, uint flags);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT3 r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
}
"@
$CHEVRON = -join (0x663E,0x793A,0x9690,0x85CF,0x7684,0x56FE,0x6807 | ForEach-Object { [char]$_ })

function Find-Flyout {
    [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
        [System.Windows.Automation.TreeScope]::Children,
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ClassNameProperty,
            "TopLevelWindowForOverflowXamlIsland")))
}

# ⓪ 关掉上次遗留的弹层
$old = Find-Flyout
if ($old) {
    try {
        $hOld = [IntPtr]$old.Current.NativeWindowHandle
        [Win3]::SetForegroundWindow($hOld) | Out-Null
        Start-Sleep -Milliseconds 150
        [System.Windows.Forms.SendKeys]::SendWait("{ESC}")
        Write-Output "sent ESC to leftover flyout"
    } catch { Write-Output ("close leftover failed: {0}" -f $_.Exception.Message) }
}

Add-Type -AssemblyName System.Windows.Forms

$h = [Win3]::FindWindow("Shell_TrayWnd", $null)
$r0 = New-Object RECT3
[Win3]::GetWindowRect($h, [ref]$r0) | Out-Null

# ① 隐藏再显示（模拟接管态借壳），等它停靠稳定
[Win3]::ShowWindow($h, 0) | Out-Null
Start-Sleep -Milliseconds 400
[Win3]::ShowWindow($h, 8) | Out-Null   # SW_SHOWNA
Start-Sleep -Milliseconds 300

# ② 已显示状态下挪出屏幕，看会不会被弹回
[Win3]::SetWindowPos($h, [IntPtr]::Zero, 0, -300, 0, 0, 0x0001 -bor 0x0002 -bor 0x0010) | Out-Null
Start-Sleep -Milliseconds 60
$r1 = New-Object RECT3
[Win3]::GetWindowRect($h, [ref]$r1) | Out-Null
Write-Output ("+60ms after move: y={0} (target -300)" -f $r1.T)
Start-Sleep -Milliseconds 600
$r2 = New-Object RECT3
[Win3]::GetWindowRect($h, [ref]$r2) | Out-Null
Write-Output ("+660ms after move: y={0}" -f $r2.T)
if ($r2.T -lt -100) { Write-Output "MOVE STICKS while visible" } else { Write-Output "MOVE REVERTED (shell re-docks)" }

# ③ 若挪出成功，验证屏幕外 XAML 是否照常渲染（UIA 枚举）
if ($r2.T -lt -100) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $chevron = $null
    try {
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
        $btns = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
        $sw.Stop()
        Write-Output ("offscreen-visible UIA buttons: {0} ({1}ms)" -f $btns.Count, $sw.ElapsedMilliseconds)
        foreach ($b in $btns) {
            try { $n = $b.Current.Name; if ($n -and $n.Contains($CHEVRON)) { $chevron = $b } } catch {}
        }
    } catch { Write-Output ("UIA FAILED: {0}" -f $_.Exception.Message) }
    Write-Output ("chevron reachable offscreen: {0}" -f ($null -ne $chevron))

    if ($chevron) {
        # ④ 挪回原位 -> Invoke -> 立即隐藏
        [Win3]::SetWindowPos($h, [IntPtr]::Zero, $r0.L, $r0.T, 0, 0, 0x0001 -bor 0x0002 -bor 0x0010) | Out-Null
        $chevron.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
        [Win3]::ShowWindow($h, 0) | Out-Null
        Start-Sleep -Milliseconds 1200
        $fly = Find-Flyout
        if ($fly) { Write-Output ("FLYOUT OK: rect={0}" -f $fly.Current.BoundingRectangle) }
        else { Write-Output "FLYOUT MISSING" }
    }
}

# ⑤ 恢复
[Win3]::SetWindowPos($h, [IntPtr]::Zero, $r0.L, $r0.T, ($r0.R - $r0.L), ($r0.B - $r0.T), 0x0002 -bor 0x0010) | Out-Null
[Win3]::ShowWindow($h, 5) | Out-Null
Start-Sleep -Milliseconds 300
Write-Output ("restored: visible={0}" -f [Win3]::IsWindowVisible($h))
