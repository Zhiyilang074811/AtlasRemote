use std::time::{Instant, Duration};
use std::ptr;
use atlas_frame::{Frame, PixelFormat, FrameSource};
use anyhow::{Result, Context};
use windows::core::Interface;
use windows::Win32::Foundation::{HWND, HANDLE};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::Common::DXGI_OUTDUPL_FRAME_INFO;
use windows::Win32::Graphics::Dxgi::{*};
use windows::Win32::Graphics::Direct3D11::{*};
use windows::Win32::Graphics::Direct3D::{*};
use windows::Win32::Graphics::Gdi::{*};
use windows::Win32::UI::WindowsAndMessaging::{*};

const ERROR_WAS_STILL_DRAWN: u32 = 0x887A0006;

pub struct DxgiCapture {
    name: String, width: u32, height: u32, fallback_gdi: bool,
    duplication: Option<IDXGIOutputDuplication>,
    device: Option<ID3D11Device>, context: Option<ID3D11DeviceContext>,
    staging_tex: Option<ID3D11Texture2D>,
    last_acquire: Option<Instant>, frame_counter: u64,
}