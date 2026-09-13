param([int]$DelaySec = 0)
Start-Sleep -Seconds $DelaySec
Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public class K { [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo); }'
[K]::keybd_event(0x5B,0,0,[UIntPtr]::Zero)   # LWIN down
Start-Sleep -Milliseconds 40
[K]::keybd_event(0x44,0,0,[UIntPtr]::Zero)   # D down
Start-Sleep -Milliseconds 40
[K]::keybd_event(0x44,0,2,[UIntPtr]::Zero)   # D up
Start-Sleep -Milliseconds 20
[K]::keybd_event(0x5B,0,2,[UIntPtr]::Zero)   # LWIN up
Write-Host "win+d injected"
