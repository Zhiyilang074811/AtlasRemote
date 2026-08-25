// DXGI Desktop Duplication Capture Backend for AtlasRemote
// Supports NVENC H264 encoding when an RTX GPU is present.

use std::time::{Instant, Duration};
use std::ptr;
use atlas_frame::{Frame, PixelFormat, FrameSource};
use atlas_codec::Codec;
use anyhow::{Result, Context};
use windows::core::Interface;
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, IDXGIFactory2, IDXGIAdapter1,
    IDXGIOutput1, IDXGIOutputDuplication,
    IDXGIResource, DXGI_OUTDUPL_FRAME_INFO,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
    ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
    D3D11_USAGE_STAGING, D3D11_MAP_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC,
};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_UNKNOWN,
    D3D_FEATURE_LEVEL_11_0,
};
use windows::Win32::Graphics::Gdi::{
    GetWindowDC, BitBlt, ReleaseDC,
    CreateCompatibleDC, CreateCompatibleBitmap,
    DeleteDC, DeleteObject,
    BITMAPINFOHEADER, BITMAPINFO, BI_RGB, GetDIBits,
    DIB_RGB_COLORS,
};
use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

const ERROR_WAS_STILL_DRAWN: u32 = 0x887A0006u32;
const ERROR_TIMEOUT: u32 = 0x887A0027u32;

pub struct DxgiCapture {
    name: String,
    width: u32,
    height: u32,
    fallback_gdi: bool,
    duplication: Option<IDXGIOutputDuplication>,
    device: Option<ID3D11Device>,
    context: Option<ID3D11DeviceContext>,
    staging_tex: Option<ID3D11Texture2D>,
    frame_counter: u64,
    prev_dirty: bool,
    pub(crate) encoder: Option<Codec>,
}

impl DxgiCapture {
    pub fn new(monitor_index: u32) -> Result<Self> {
        unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
        match Self::dxgi_init(monitor_index) {
            Ok(cap) => {
                eprintln!("[CAPTURE] DXGI OK: {} {}x{}", cap.name, cap.width, cap.height);
                return Ok(cap);
            }
            Err(e) => {
                eprintln!("[CAPTURE] DXGI failed: {}, trying GDI fallback", e);
            }
        }
        Self::new_gdi()
    }

    fn dxgi_init(monitor_index: u32) -> Result<Self> {
        let factory: IDXGIFactory2 = unsafe {
            CreateDXGIFactory2(0).context("CreateDXGIFactory2 failed")?
        };
        let adapter: IDXGIAdapter1 = unsafe {
            factory.EnumAdapters1(0).context("EnumAdapters1(0) failed")?
        };
        let mut desc = unsafe { std::mem::zeroed() };
        unsafe { adapter.GetDesc1(&mut desc) }.context("GetDesc1 failed")?;
        let adapter_name = String::from_utf16_lossy(&desc.Description)
            .trim_end_matches('\0').to_string();
        eprintln!("[CAPTURE] Adapter: {}", adapter_name);

        let output = unsafe {
            adapter.EnumOutputs(monitor_index)
                .context(format!("EnumOutputs({}) failed", monitor_index))?
        };
        let output1: IDXGIOutput1 = output.cast::<IDXGIOutput1>()
            .context("Cast to IDXGIOutput1 failed")?;

        let mut out_desc = unsafe { std::mem::zeroed() };
        unsafe { output.GetDesc(&mut out_desc) }.context("GetDesc failed")?;
        let r = out_desc.DesktopCoordinates;
        let width = (r.right - r.left).max(1) as u32;
        let height = (r.bottom - r.top).max(1) as u32;
        eprintln!("[CAPTURE] Resolution: {}x{}", width, height);

        let (device, context) = {
            let mut dev: Option<ID3D11Device> = None;
            let mut ctx: Option<ID3D11DeviceContext> = None;
            let base: &windows::Win32::Graphics::Dxgi::IDXGIAdapter = &adapter;
            unsafe {
                D3D11CreateDevice(
                    Some(base), D3D_DRIVER_TYPE_UNKNOWN, None,
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
                    Some(&[D3D_FEATURE_LEVEL_11_0]),
                    windows::Win32::Graphics::Direct3D11::D3D11_SDK_VERSION,
                    Some(&mut dev), None, Some(&mut ctx),
                ).map_err(|e| anyhow::anyhow!("D3D11CreateDevice failed: {}", e))?;
            }
            (dev.context("Device is None")?, ctx.context("Context is None")?)
        };

        let duplication = unsafe {
            output1.DuplicateOutput(&device).context("DuplicateOutput failed")?
        };
        let staging = Self::create_staging(&device, width, height)?;

        let mut codec = Codec::new();
        let encoder = match codec.init_nvenc(&device, &context, width, height) {
            Ok(_) => {
                eprintln!("[CAPTURE] NVENC initialized: {}x{}", width, height);
                Some(codec)
            }
            Err(e) => {
                eprintln!("[CAPTURE] NVENC init failed: {}, using BGRA fallback", e);
                None
            }
        };

        Ok(Self {
            name: adapter_name, width, height, fallback_gdi: false,
            duplication: Some(duplication),
            device: Some(device), context: Some(context),
            staging_tex: Some(staging), frame_counter: 0, encoder,
        })
    }

    fn new_gdi() -> Result<Self> {
        let width = 1920;
        let height = 1080;
        eprintln!("[CAPTURE] GDI fallback: {}x{}", width, height);
        Ok(Self {
            name: format!("GDI {}x{}", width, height),
            width, height, fallback_gdi: true,
            duplication: None, device: None, context: None,
            staging_tex: None, frame_counter: 0, encoder: None,
        })
    }

    fn create_staging(device: &ID3D11Device, width: u32, height: u32) -> Result<ID3D11Texture2D> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: width, Height: height, MipLevels: 1, ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_STAGING, BindFlags: 0,
            CPUAccessFlags: 1 << 17, MiscFlags: 0,
        };
        let mut tex: Option<ID3D11Texture2D> = None;
        unsafe {
            device.CreateTexture2D(&desc, None, Some(ptr::addr_of_mut!(tex) as *mut _))
                .context("CreateTexture2D failed")?;
        }
        tex.ok_or_else(|| anyhow::anyhow!("CreateTexture2D returned None"))
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }

    fn acquire_dxgi(&mut self) -> Result<Vec<u8>> {
        let dup = self.duplication.as_ref().context("No duplication")?;
        let ctx = self.context.as_ref().context("No context")?;
        let staging = self.staging_tex.as_ref().context("No staging")?;

        let mut res: Option<IDXGIResource> = None;
        loop {
            let mut fi: DXGI_OUTDUPL_FRAME_INFO = unsafe { std::mem::zeroed() };
            match unsafe { dup.AcquireNextFrame(100, &mut fi, &mut res) } {
                Ok(_) => break,
                Err(e) if e.code().0 == ERROR_WAS_STILL_DRAWN as i32 => {
                    std::thread::sleep(Duration::from_millis(1)); continue;
                }
                Err(e) if e.code().0 == ERROR_TIMEOUT as i32 => {
                    return Err(anyhow::anyhow!("AcquireNextFrame timeout"));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("AcquireNextFrame: {} (0x{:08X})", e, e.code().0));
                }
            }
        }

        let surface: ID3D11Texture2D = res
            .context("No resource from AcquireNextFrame")?
            .cast::<ID3D11Texture2D>()
            .context("Cast IDXGIResource to ID3D11Texture2D failed")?;

        unsafe { ctx.CopyResource(staging, &surface); }

        let mut mapped: D3D11_MAPPED_SUBRESOURCE = unsafe { std::mem::zeroed() };
        unsafe {
            ctx.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .context("Map failed")?;
        }

        let stride = mapped.RowPitch as usize;
        let src = mapped.pData as *const u8;
        let mut data = vec![0u8; (self.height as usize) * (self.width as usize) * 4];
        unsafe {
            for row in 0..self.height {
                let s = src.add(row as usize * stride);
                let d = data.as_mut_ptr().add((row as usize) * (self.width as usize * 4));
                ptr::copy_nonoverlapping(s, d, self.width as usize * 4);
            }
        }
        unsafe { ctx.Unmap(staging, 0); }
        // Dirty region detection - check BEFORE releasing frame
        let has_change = fi.TotalRects > 0;
        self.prev_dirty = has_change;
        if !has_change {
            unsafe { dup.ReleaseFrame().ok(); }
            self.frame_counter += 1;
            eprintln!("[CAPTURE] Frame {} STATIC - skipping", self.frame_counter);
            return Ok(vec![0u8; 0]);
        }
        unsafe { dup.ReleaseFrame().ok(); }
        self.frame_counter += 1;
        Ok(data)
    }

    fn acquire_gdi(&mut self) -> Result<Vec<u8>> {
        let hw = unsafe { GetDesktopWindow() };
        let hdc_src = unsafe { GetWindowDC(hw) };
        if hdc_src.0 == 0 { return Err(anyhow::anyhow!("GetWindowDC failed")); }
        let hdc_mem = unsafe { CreateCompatibleDC(hdc_src) };
        if hdc_mem.0 == 0 { unsafe { ReleaseDC(hw, hdc_src) }; return Err(anyhow::anyhow!("CreateCompatibleDC failed")); }
        let hbm = unsafe { CreateCompatibleBitmap(hdc_src, self.width as i32, self.height as i32) };
        if hbm.0 == 0 { unsafe { let _ = DeleteDC(hdc_mem); } unsafe { ReleaseDC(hw, hdc_src) }; return Err(anyhow::anyhow!("CreateCompatibleBitmap failed")); }
        let _old = unsafe { windows::Win32::Graphics::Gdi::SelectObject(hdc_mem, hbm) };
        let ok = unsafe { BitBlt(hdc_mem, 0, 0, self.width as i32, self.height as i32, hdc_src, 0, 0, windows::Win32::Graphics::Gdi::SRCCOPY) };
        if !ok.is_ok() { unsafe { ReleaseDC(hw, hdc_src) }; return Err(anyhow::anyhow!("BitBlt failed")); }
        let mut bmi: BITMAPINFO = unsafe { std::mem::zeroed() };
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = self.width as i32;
        bmi.bmiHeader.biHeight = -(self.height as i32);
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;
        bmi.bmiHeader.biSizeImage = 0;
        let mut bgra = vec![0u8; (self.width as usize) * (self.height as usize) * 4];
        let lines = unsafe {
            GetDIBits(hdc_mem, hbm, 0, self.height, Some(bgra.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS)
        };
        if lines == 0 { unsafe { ReleaseDC(hw, hdc_src) }; return Err(anyhow::anyhow!("GetDIBits failed")); }
        unsafe { ReleaseDC(hw, hdc_src); let _ = DeleteObject(hbm); let _ = DeleteDC(hdc_mem); }
        Ok(bgra)
    }
}

impl Drop for DxgiCapture {
    fn drop(&mut self) {
        if let Some(ref dup) = self.duplication {
            unsafe { dup.ReleaseFrame().ok(); }
        }
    }
}

impl FrameSource for DxgiCapture {
    fn next_frame(&mut self) -> Option<Frame> {
        let start = Instant::now();
        let data = if self.fallback_gdi {
            match self.acquire_gdi() { Ok(d) => d, Err(e) => { eprintln!("[CAPTURE] GDI err: {}", e); return None; } }
        } else {
            match self.acquire_dxgi() { Ok(d) => d, Err(e) => { eprintln!("[CAPTURE] DXGI err: {}", e); return None; } }
        };
        eprintln!("[CAPTURE] Frame {} {}x{} {}ms", self.frame_counter, self.width, self.height, start.elapsed().as_millis());
        Some(Frame { timestamp: chrono::Utc::now().timestamp_micros() as u64, width: self.width, height: self.height, format: PixelFormat::Bgra32, data, frame_id: self.frame_counter })
    }
    fn name(&self) -> &str { &self.name }
}
