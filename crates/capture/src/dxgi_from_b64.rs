// DXGI Desktop Duplication Capture Backend for AtlasRemote
// Falls back to GDI if DXGI is unavailable.

use std::time::{Instant, Duration};

use atlas_frame::{Frame, PixelFormat, FrameSource};
use anyhow::{Result, Context};

use windows::core::Interface;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct DxgiCapture {
    name: String,
    width: u32,
    height: u32,
    fallback_gdi: bool,
    factory: Option<IDXGIFactory2>,
    adapter: Option<IDXGIAdapter1>,
    duplication: Option<IDXGIOutputDuplication>,
    device: Option<ID3D11Device>,
    context: Option<ID3D11DeviceContext>,
    staging_tex: Option<ID3D11Texture2D>,
    last_acquire: Option<Instant>,
    release_count: u32,
}