# 实验四：托盘弹层窗口是否常驻（隐藏不销毁）？
# 若常驻：任务栏隐藏状态下直接 ShowWindow(弹层) 能否正常显示、图标是否新鲜？
# 若可行 -> 借壳流程只需跑一次，之后零闪现。
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public struct RECT4 { public int L, T, R, B; }
public class Win4 {
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string cls, string title);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
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

# ① 当前弹层状态（实验二遗留的，可能还开着）
$fly = Find-Flyout
if ($fly) {
    Write-Output ("flyout exists: visible={0} rect={1}" -f $fly.Current.IsOffscreen, $fly.Current.BoundingRectangle)
    # 用 Esc 关掉它（此时它若在前台）
    try {
        $hF = [IntPtr]$fly.Current.NativeWindowHandle
        [Win4]::SetForegroundWindow($hF) | Out-Null
        Start-Sleep -Milliseconds 150
        [System.Windows.Forms.SendKeys]::SendWait("{ESC}")
        Start-Sleep -Milliseconds 400
        Write-Output ("after ESC: visible={0}" -f (-not $fly.Current.IsOffscreen))
    } catch { Write-Output ("esc failed: {0}" -f $_.Exception.Message) }
} else {
    Write-Output "flyout window NOT found (not persistent or already gone)"
}

# ② 弹层关闭后窗口是否还在（常驻检查）？
Start-Sleep -Milliseconds 400
$fly2 = Find-Flyout
if ($fly2) {
    Write-Output ("flyout PERSISTS after dismiss: offscreen={0}" -f $fly2.Current.IsOffscreen)
} else {
    Write-Output "flyout GONE after dismiss (destroyed on close)"
}

# ③ 任务栏隐藏状态下直接 ShowWindow(弹层)，图标是否还在/新鲜
$h = [Win4]::FindWindow("Shell_TrayWnd", $null)
$r0 = New-Object RECT4
[Win4]::GetWindowRect($h, [ref]$r0) | Out-Null
[Win4]::ShowWindow($h, 0) | Out-Null
Start-Sleep -Milliseconds 500

if ($fly2) {
    $hF2 = [IntPtr]$fly2.Current.NativeWindowHandle
    [Win4]::ShowWindow($hF2, 8) | Out-Null   # SW_SHOWNA
    Start-Sleep -Milliseconds 800
    $fly3 = Find-Flyout
    if ($fly3 -and -not $fly3.Current.IsOffscreen) {
        # 数一下里面有多少按钮（托盘图标）
        $inner = $fly3.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
        Write-Output ("flyout RESHOWED while taskbar hidden: rect={0} innerElements={1}" -f $fly3.Current.BoundingRectangle, $inner.Count)
    } else {
        Write-Output "flyout reshow failed (window won't show or closed itself)"
    }
}

# ④ 恢复任务栏
[Win4]::ShowWindow($h, 5) | Out-Null
Start-Sleep -Milliseconds 300
Write-Output ("restored taskbar: visible={0}" -f [Win4]::IsWindowVisible($h))
