# 实验：任务栏 SW_HIDE 隐藏状态下，UIA 能否枚举到「显示隐藏的图标」chevron
# 并 Invoke 唤出原生托盘弹层（TopLevelWindowForOverflowXamlIsland）。
# 结论决定 open_overflow 能否完全不显示任务栏（零闪现）。
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win {
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
}
"@

$h = [Win]::FindWindow("Shell_TrayWnd", $null)
Write-Output ("taskbar hwnd: {0}, visible: {1}" -f $h, [Win]::IsWindowVisible($h))
if ($h -eq [IntPtr]::Zero) { exit 1 }

# 模拟桌面接管：隐藏任务栏
[Win]::ShowWindow($h, 0) | Out-Null
Start-Sleep -Milliseconds 700
Write-Output ("after SW_HIDE, visible: {0}" -f [Win]::IsWindowVisible($h))

$chevron = $null
try {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
    Write-Output "UIA: FromHandle OK on hidden taskbar"
    $btns = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    Write-Output ("hidden taskbar UIA buttons: {0}" -f $btns.Count)
    foreach ($b in $btns) {
        try {
            $n = $b.Current.Name
            if ($n -and $n.Contains("显示隐藏的图标")) {
                $chevron = $b
                Write-Output "FOUND chevron while hidden"
            }
        } catch {}
    }
} catch {
    Write-Output ("UIA: FAILED on hidden taskbar: {0}" -f $_.Exception.Message)
}

if ($chevron) {
    try {
        $inv = $chevron.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $inv.Invoke()
        Write-Output "chevron Invoke() sent"
        Start-Sleep -Milliseconds 1000
        $fly = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
            [System.Windows.Automation.TreeScope]::Children,
            (New-Object System.Windows.Automation.PropertyCondition(
                [System.Windows.Automation.AutomationElement]::ClassNameProperty,
                "TopLevelWindowForOverflowXamlIsland")))
        if ($fly) {
            Write-Output ("FLYOUT APPEARED while taskbar hidden: '{0}' rect={1}" -f $fly.Current.Name, $fly.Current.BoundingRectangle)
        } else {
            Write-Output "flyout NOT found (invoke had no visible effect)"
        }
    } catch {
        Write-Output ("Invoke failed: {0}" -f $_.Exception.Message)
    }
} else {
    Write-Output "chevron NOT reachable while hidden -> 零闪现方案不可行"
}

# 恢复任务栏
[Win]::ShowWindow($h, 5) | Out-Null
Start-Sleep -Milliseconds 400
Write-Output ("restored, visible: {0}" -f [Win]::IsWindowVisible($h))
