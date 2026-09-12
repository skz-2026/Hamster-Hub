//! 主音量控制（渲染端点 IAudioEndpointVolume）：
//! get 返回当前电平与静音，set 同时落电平与静音。COM 每调用初始化/反初始化，无共享状态。

use windows::core::Error as WinError;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VolumeState {
    /// 0.0 - 1.0
    pub level: f32,
    pub muted: bool,
}

fn with_endpoint<T>(
    f: impl FnOnce(&IAudioEndpointVolume) -> windows::core::Result<T>,
) -> Result<T, String> {
    unsafe {
        // 线程可能已被其它库初始化为其它模式（RPC_E_CHANGED_MODE）——不影响我们使用
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        let coinit_ok = hr.is_ok();
        let result = (|| {
            let enum_: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|e: WinError| e.to_string())?;
            let dev = enum_
                .GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(|e| e.to_string())?;
            // Activate 是文档规定获取端点服务的方式；部分驱动设备的 QueryInterface
            // 不应答 IAudioEndpointVolume（真机踩坑 E_NOINTERFACE）
            let vol: IAudioEndpointVolume =
                dev.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
            f(&vol).map_err(|e| e.to_string())
        })();
        if coinit_ok {
            CoUninitialize();
        }
        result
    }
}

pub fn get() -> Result<VolumeState, String> {
    with_endpoint(|vol| unsafe {
        let level = vol.GetMasterVolumeLevelScalar()?;
        let muted = vol.GetMute()?.as_bool();
        Ok(VolumeState { level, muted })
    })
}

pub fn set(level: f32, muted: bool) -> Result<(), String> {
    let level = level.clamp(0.0, 1.0);
    with_endpoint(|vol| unsafe {
        vol.SetMasterVolumeLevelScalar(level, std::ptr::null())?;
        vol.SetMute(muted, std::ptr::null())?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真机音量往返：设置 50% → 读回应为 50%，随后还原。`--ignored` 手动跑。
    #[test]
    #[ignore]
    fn volume_roundtrip_real() {
        let orig = get().expect("读取音量失败");
        set(0.5, false).expect("设置音量失败");
        let v = get().expect("回读音量失败");
        assert!((v.level - 0.5).abs() < 0.02, "回读 {} ≠ 0.5", v.level);
        set(orig.level, orig.muted).expect("还原音量失败");
    }
}
