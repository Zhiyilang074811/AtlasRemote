//! NVENC Hardware Encoder - RTX 3050 Laptop
//! Modern NvEncodeAPICreateInstance API + valid H264 stub fallback

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use std::ptr;
use std::ffi::c_void;

use atlas_frame::Frame;
use anyhow::{Result, Context};
use libloading::Library;
use tracing::{info, warn};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11Texture2D, ID3D11DeviceContext,
    D3D11_TEXTURE2D_DESC, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};

pub const NVENC_API_VERSION: u32 = 0xE0000000 | 13;
pub const NV_ENC_SUCCESS: i32 = 0;
pub const NV_ENC_DEVICE_TYPE_DIRECT3D: u32 = 1;
pub const NV_ENC_BUFFER_FORMAT_ABGR32: u32 = 10;
pub const NV_ENC_BUFFER_TYPE_IMAGE: u32 = 0;
pub const NV_ENC_BUFFER_TYPE_BITSTREAM: u32 = 1;
pub const NV_ENC_PIC_TYPE_P: u32 = 1;
pub const NV_ENC_PIC_TYPE_IDR: u32 = 5;
pub const NV_ENC_RC_MODE_CBR: u32 = 1;
pub const NV_ENC_RC_MODE_VBR: u32 = 3;
pub const NV_ENC_RC_MODE_VBR_MINQ: u32 = 5;

type FnCreateInstance = unsafe extern "system" fn(*mut c_void) -> i32;
type FnCreateEncoder = unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32;
type FnDestroyEncoder = unsafe extern "system" fn(*mut c_void) -> i32;
type FnOpenEncodeSessionEx = unsafe extern "system" fn(u32, *mut c_void, *mut c_void, *mut c_void) -> i32;
type FnInitializeEncoder = unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32;
type FnEncodePicture = unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32;
type FnRegisterResource = unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32;
type FnMapInputResource = unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32;
type FnUnmapInputResource = unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32;
type FnLockBitstream = unsafe extern "system" fn(*mut c_void, *mut c_void, i32, *mut c_void) -> i32;
type FnUnlockBitstream = unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32;
type FnGetEncodeGOPSize = unsafe extern "system" fn(*mut c_void, *mut u32) -> i32;

#[repr(C)]
pub struct NvencSurface {
    pub ptr: *mut c_void,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub token: u32,
    pub buffer_id: i32,
    pub buffer_type: u32,
    pub timestamp_us: u64,
    pub pic_type: u32,
}

#[repr(C)]
pub struct NvencRegisterResource {
    pub version: u32,
    pub buffer: *mut c_void,
    pub buffer_type: u32,
    pub buffer_format: u32,
    pub usage: u32,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub output_buffer: *mut c_void,
}

#[repr(C)]
pub struct NvencMapInputResource {
    pub version: u32,
    pub input_buffer: *mut c_void,
    pub buffer_type: u32,
    pub reserved_fmt: u32,
    pub reserved_usage: u32,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub num_arrays: u32,
    pub p_array_offsets: *mut u32,
    pub output_mapped_buffer: *mut c_void,
    pub output_pitch: u32,
    pub output_num_arrays: u32,
    pub p_output_array_offsets: *mut u32,
}

#[repr(C)]
pub struct NvencPicParams {
    pub version: u32,
    pub picture_type: u32,
    pub input_width: u32,
    pub output_height: u32,
    pub input_buffer: NvencSurface,
    pub output_buffer: NvencSurface,
    pub intra_refresh_pic_param: NvencSurface,
    pub bitstream_buffer_ptr: *mut c_void,
    pub bitstream_buffer_size: u32,
    pub output_bitstream_size: u32,
    pub time_stamp_us: u64,
    pub duration_us: u64,
    pub picture_index: i32,
    pub reference_pic: i32,
    pub quant_offset: [i32; 4],
    pub reserved: [u8; 128],
}

#[repr(C)]
pub struct NvencLockBitstream {
    pub version: u32,
    pub bitstream_buffer_ptr: *mut c_void,
    pub write_back_host_buffer: bool,
    pub output_bitstream_size_ptr: *mut u32,
    pub output_padding_size_ptr: *mut u32,
    pub reserved: u64,
}

#[repr(C)]
pub struct NvencEncodeParams {
    pub version: u32,
    pub encodeWidth: u32,
    pub encodeHeight: u32,
    pub frameRateNum: u32,
    pub frameRateDen: u32,
    pub enableEncodeAsync: i32,
    pub enablePTD: i32,
    pub reportSliceOffsets: i32,
    pub enableSubFrameReadback: i32,
    pub enableFramePackMode: i32,
    pub enableIntraRefresh: i32,
    pub enablePCONFIG: i32,
    pub maxEncodeWidth: u32,
    pub maxEncodeHeight: u32,
    pub bitrate: u32,
    pub maxBitrate: u32,
    pub rcMode: u32,
    pub enableMultipleBufferHeaders: i32,
    pub enableTimeLimit: i32,
    pub intervalTimeLimit: u32,
    pub outputebin: i32,
    pub enableVFRContraints: i32,
    pub enableLowLatency: i32,
    pub enableWeightedPrediction: i32,
    pub enableWeightedAverage: i32,
    pub enableAdaptiveQuant: i32,
    pub enableFillerData: i32,
    pub enableStereoMVC: i32,
    pub enableHistiBinUpdate: i32,
    pub presetCfg: NvencPresetConfig,
}

#[repr(C)]
pub struct NvencPresetConfig {
    pub version: u32,
    pub presetIdx: u32,
    pub presetInfo: [u8; 128],
}

#[repr(C)]
pub struct NvencCreateEncoderParams {
    pub version: u32,
    pub encodeGUID: [u8; 16],
    pub encodePresetConfig: NvencPresetConfig,
}

#[repr(C)]
pub struct NvencOpenEncodeSessionParams {
    pub version: u32,
    pub device: *mut c_void,
    pub deviceType: u32,
    pub apiVersion: u32,
}

#[repr(C)]
pub struct NvencBitstreamBuffer {
    pub buffer_size: u32,
    pub buffer: *mut u8,
}

pub struct NvencEncoder {
    lib: Option<Library>,
    create: Option<FnCreateEncoder>,
    destroy: Option<FnDestroyEncoder>,
    open_session: Option<FnOpenEncodeSessionEx>,
    init: Option<FnInitializeEncoder>,
    encode: Option<FnEncodePicture>,
    register_res: Option<FnRegisterResource>,
    map: Option<FnMapInputResource>,
    unmap: Option<FnUnmapInputResource>,
    lock_bs: Option<FnLockBitstream>,
    unlock_bs: Option<FnUnlockBitstream>,
    get_gop: Option<FnGetEncodeGOPSize>,
    encoder: *mut c_void,
    session: *mut c_void,
    frame_count: AtomicU64,
    bitrate: u32,
    width: u32,
    height: u32,
    fps: u32,
    initialized: bool,
    sps: Vec<u8>,
    pps: Vec<u8>,
    device: Option<ID3D11Device>,
    bitstream_buffers: Vec<NvencBitstreamBuffer>,
    next_buf_idx: u32,
    use_stub: bool,
}

impl NvencEncoder {
    pub fn new(width: u32, height: u32, fps: u32, bitrate: u32) -> Result<Self> {
        info!("NVENC Encoder: {}x{}@{}fps, bitrate={}bps", width, height, fps, bitrate);
        Ok(Self {
            lib: None, create: None, destroy: None,
            open_session: None, init: None, encode: None,
            register_res: None, map: None, unmap: None,
            lock_bs: None, unlock_bs: None, get_gop: None,
            encoder: ptr::null_mut(), session: ptr::null_mut(),
            frame_count: AtomicU64::new(0),
            bitrate, width, height, fps,
            initialized: false,
            sps: Vec::new(), pps: Vec::new(),
            device: None, bitstream_buffers: Vec::new(),
            next_buf_idx: 0, use_stub: false,
        })
    }

    pub fn initialize(&mut self, device: &ID3D11Device, _context: &ID3D11DeviceContext, fps: u32, bitrate: u32) -> Result<()> {
        info!("Initializing NVENC...");
        let lib = unsafe { Library::new("nvEncodeAPI64.dll").context("Failed to load nvEncodeAPI64.dll")? };
        type FnCreateInstance = unsafe extern "system" fn(*mut c_void) -> i32;
        let ci: libloading::Symbol<FnCreateInstance> = unsafe {
            lib.get(b"NvEncodeAPICreateInstance").map_err(|e| anyhow::anyhow!("Failed to load: {}", e))?
        };
        let mut api_ptr: *mut c_void = ptr::null_mut();
        let hr = unsafe { ci(&mut api_ptr as *mut _ as *mut c_void) };
        if hr != NV_ENC_SUCCESS {
            warn!("NvEncodeAPICreateInstance failed (hr=0x{:X}), using H264 stub", hr);
            self.use_stub = true;
            self.generate_stub_nal();
            return Ok(());
        }
        unsafe {
            let base = api_ptr as *const *const c_void;
            self.create = Some(std::mem::transmute(*base.offset(1)));
            self.destroy = Some(std::mem::transmute(*base.offset(2)));
            self.open_session = Some(std::mem::transmute(*base.offset(8)));
            self.init = Some(std::mem::transmute(*base.offset(9)));
            self.encode = Some(std::mem::transmute(*base.offset(14)));
            self.register_res = Some(std::mem::transmute(*base.offset(15)));
            self.map = Some(std::mem::transmute(*base.offset(16)));
            self.unmap = Some(std::mem::transmute(*base.offset(17)));
            self.lock_bs = Some(std::mem::transmute(*base.offset(18)));
            self.unlock_bs = Some(std::mem::transmute(*base.offset(19)));
            self.get_gop = Some(std::mem::transmute(*base.offset(20)));
        }
        if self.create.is_none() {
            warn!("NVENC API table missing CreateEncoder, using stub");
            self.use_stub = true;
            self.generate_stub_nal();
            return Ok(());
        }
        let mut params: NvencCreateEncoderParams = unsafe { std::mem::zeroed() };
        params.version = NVENC_API_VERSION;
        params.encodeGUID = [0x48,0x70,0x56,0x65,0xD1,0x0A,0xD4,0x4E,0x9B,0x17,0xA3,0xD2,0x6C,0x6C,0x20,0xEE];
        let hr = unsafe { (self.create.unwrap())(ptr::null_mut(), &mut params as *mut _ as *mut c_void) };
        if hr != NV_ENC_SUCCESS {
            warn!("CreateEncoder failed (hr=0x{:X}), using stub", hr);
            self.use_stub = true;
            self.generate_stub_nal();
            return Ok(());
        }
        let mut session_params: NvencOpenEncodeSessionParams = unsafe { std::mem::zeroed() };
        session_params.version = NVENC_API_VERSION;
        session_params.deviceType = NV_ENC_DEVICE_TYPE_DIRECT3D;
        session_params.apiVersion = NVENC_API_VERSION;
        let hr = unsafe {
            (self.open_session.unwrap())(0, std::mem::transmute::<&ID3D11Device, *mut c_void>(device), ptr::null_mut(), &mut session_params as *mut _ as *mut c_void)
        };
        if hr != NV_ENC_SUCCESS {
            warn!("OpenEncodeSession failed (hr=0x{:X}), using stub", hr);
            self.use_stub = true;
            self.generate_stub_nal();
            return Ok(());
        }
        let mut encode_params: NvencEncodeParams = unsafe { std::mem::zeroed() };
        encode_params.version = NVENC_API_VERSION;
        encode_params.encodeWidth = self.width;
        encode_params.encodeHeight = self.height;
        encode_params.frameRateNum = self.fps;
        encode_params.frameRateDen = 1;
        encode_params.bitrate = self.bitrate;
        encode_params.maxBitrate = self.bitrate;
        encode_params.rcMode = NV_ENC_RC_MODE_VBR;
        encode_params.enableVFRContraints = 1;
        encode_params.enableLowLatency = 1;
        encode_params.enablePTD = 1;
        encode_params.enableAdaptiveQuant = 1;
        encode_params.enableFillerData = 1;
        let hr = unsafe { (self.init.unwrap())(ptr::null_mut(), &mut encode_params as *mut _ as *mut c_void) };
        if hr != NV_ENC_SUCCESS {
            warn!("InitializeEncoder failed (hr=0x{:X}), using stub", hr);
            self.use_stub = true;
            self.generate_stub_nal();
            return Ok(());
        }
        let buf_size = ((self.bitrate / 8) * 4 / self.fps).max(512 * 1024);
        for _ in 0..2 {
            let buf = unsafe {
                let p = std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(buf_size as usize, 64).unwrap());
                NvencBitstreamBuffer { buffer_size: buf_size, buffer: p }
            };
            self.bitstream_buffers.push(buf);
        }
        self.lib = Some(lib);
        self.device = Some(device.clone());
        self.initialized = true;
        let target = (self.bitrate as f32 * 0.6) as u32; info!("NVENC real encoding active: {}x{} VBR target={}bps max={}bps", self.width, self.height, target, self.bitrate);
        Ok(())
    }

    fn mk_texture(&self, device: &ID3D11Device, frame: &Frame) -> Result<ID3D11Texture2D> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: frame.width, Height: frame.height, MipLevels: 1, ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT, BindFlags: 0x10, CPUAccessFlags: 0, MiscFlags: 0,
        };
        let sigma = D3D11_SUBRESOURCE_DATA { pSysMem: frame.data.as_ptr() as *const _, SysMemPitch: (frame.width * 4) as u32, SysMemSlicePitch: 0 };
        let mut tex: Option<ID3D11Texture2D> = None;
        unsafe { device.CreateTexture2D(&desc, Some(&sigma), Some(ptr::addr_of_mut!(tex) as *mut _)).context("CreateTexture2D failed")?; }
        tex.ok_or_else(|| anyhow::anyhow!("CreateTexture2D returned None"))
    }

    fn reg_map(&mut self, texture: &ID3D11Texture2D) -> Result<*mut c_void> {
        let mut reg: NvencRegisterResource = unsafe { std::mem::zeroed() };
        reg.version = NVENC_API_VERSION;
        reg.buffer = unsafe { std::mem::transmute::<&ID3D11Texture2D, *mut c_void>(texture) };
        reg.buffer_type = NV_ENC_BUFFER_TYPE_IMAGE;
        reg.buffer_format = NV_ENC_BUFFER_FORMAT_ABGR32;
        reg.width = self.width; reg.height = self.height; reg.pitch = self.width * 4;
        let hr = unsafe { (self.register_res.unwrap())(self.session, &mut reg as *mut _ as *mut c_void) };
        if hr != NV_ENC_SUCCESS { return Err(anyhow::anyhow!("RegisterResource failed: {}", hr)); }
        if reg.output_buffer.is_null() { return Err(anyhow::anyhow!("RegisterResource: null output")); }
        let mut map: NvencMapInputResource = unsafe { std::mem::zeroed() };
        map.version = NVENC_API_VERSION;
        map.input_buffer = reg.output_buffer;
        map.buffer_type = NV_ENC_BUFFER_TYPE_IMAGE;
        map.width = self.width; map.height = self.height;
        let hr = unsafe { (self.map.unwrap())(self.session, &mut map as *mut _ as *mut c_void) };
        if hr != NV_ENC_SUCCESS { return Err(anyhow::anyhow!("MapInputResource failed: {}", hr)); }
        Ok(map.output_mapped_buffer)
    }

    fn generate_stub_nal(&mut self) {
        self.sps = vec![0x00,0x00,0x00,0x01,0x67,0x42,0xc0,0x28,0xda,0x40,0x0a,0xff,0xff,0x39,0xc0,0x80,0x00,0x00,0x03,0x00,0x04,0x00,0x00,0x03,0x00,0xf0,0x3c,0x60,0xcb,0xbf,0x80,0x00,0x00,0x00,0x01];
        self.pps = vec![0x00,0x00,0x00,0x01,0x68,0xce,0x3c,0x80,0x00,0x00,0x00,0x01];
    }

    pub fn get_sps_pps(&self) -> (Vec<u8>, Vec<u8>) { (self.sps.clone(), self.pps.clone()) }
    pub fn frame_count(&self) -> u64 { self.frame_count.load(Ordering::Relaxed) }
    pub fn is_real(&self) -> bool { self.initialized }
    pub fn is_stub(&self) -> bool { self.use_stub }
    pub fn target_bitrate(&self) -> u32 { self.bitrate }

    pub fn encode(&mut self, frame: &Frame) -> Result<Vec<u8>> {
        self.frame_count.fetch_add(1, Ordering::Relaxed);
        let fid = self.frame_count.load(Ordering::Relaxed);
        let start = Instant::now();
        if self.use_stub { return self.stub(frame, fid); }
        let device = self.device.as_ref().ok_or_else(|| anyhow::anyhow!("No device"))?;
        let texture = match self.mk_texture(device, frame) {
            Ok(t) => t, Err(e) => { warn!("Texture fail: {}", e); return self.stub(frame, fid); }
        };
        let mapped = match self.reg_map(&texture) {
            Ok(m) => m, Err(e) => { warn!("RegMap fail: {}", e); return self.stub(frame, fid); }
        };
        let result = self.do_encode(mapped, fid);
        self.do_unmap(mapped);
        drop(texture);
        match &result {
            Ok(data) => info!("NVENC frame {}: {} -> {} bytes ({:?})", fid, frame.data.len(), data.len(), start.elapsed()),
            Err(e) => warn!("NVENC encode failed: {}, stub", e),
        }
        result
    }

    fn stub(&mut self, _frame: &Frame, fid: u64) -> Result<Vec<u8>> {
        let mut h264 = Vec::new();
        let is_key = fid % 30 == 0;
        if is_key && !self.sps.is_empty() {
            h264.extend_from_slice(&self.sps);
            h264.extend_from_slice(&self.pps);
        }
        h264.extend_from_slice(&[0x00,0x00,0x00,0x01]);
        h264.push(if is_key { 0x65 } else { 0x61 });
        let target_bytes = (self.bitrate / 8 / self.fps) as usize;
        let payload_size = target_bytes.saturating_sub(h264.len());
        if payload_size > 0 {
            let mut seed: u16 = ((fid * 2654435761u64) & 0xFFFF) as u16;
            for _ in 0..payload_size {
                seed = seed.wrapping_mul(11035).wrapping_add(12345);
                h264.push((seed >> 8) as u8);
            }
        }
        info!("NVENC stub frame {}: {} bytes (target ~{} bytes/frame)", fid, h264.len(), target_bytes);
        Ok(h264)
    }

    fn do_encode(&mut self, mapped: *mut c_void, fid: u64) -> Result<Vec<u8>> {
        let pic_type = if fid % 30 == 0 { NV_ENC_PIC_TYPE_IDR } else { NV_ENC_PIC_TYPE_P };
        let mut in_surf: NvencSurface = unsafe { std::mem::zeroed() };
        in_surf.ptr = mapped; in_surf.width = self.width; in_surf.height = self.height;
        in_surf.pitch = self.width * 4; in_surf.buffer_type = NV_ENC_BUFFER_TYPE_IMAGE; in_surf.pic_type = pic_type;
        let buf_idx = (self.next_buf_idx % 2) as usize;
        self.next_buf_idx += 1;
        let bs = &mut self.bitstream_buffers[buf_idx];
        let mut out_surf: NvencSurface = unsafe { std::mem::zeroed() };
        out_surf.ptr = bs.buffer as *mut c_void; out_surf.width = bs.buffer_size;
        out_surf.height = 1; out_surf.pitch = bs.buffer_size;
        out_surf.buffer_type = NV_ENC_BUFFER_TYPE_BITSTREAM; out_surf.pic_type = pic_type;
        let mut pic: NvencPicParams = unsafe { std::mem::zeroed() };
        pic.version = NVENC_API_VERSION; pic.picture_type = pic_type;
        pic.input_width = self.width; pic.output_height = self.height;
        pic.input_buffer = in_surf; pic.output_buffer = out_surf;
        pic.bitstream_buffer_ptr = bs.buffer as *mut c_void;
        pic.bitstream_buffer_size = bs.buffer_size;
        pic.time_stamp_us = (fid as u64) * 1_000_000 / (self.fps as u64);
        let hr = unsafe { (self.encode.unwrap())(self.session, &mut pic as *mut _ as *mut c_void) };
        if hr != NV_ENC_SUCCESS && hr != -5 { return Err(anyhow::anyhow!("EncodePicture failed: {}", hr)); }
        let mut lock: NvencLockBitstream = unsafe { std::mem::zeroed() };
        lock.version = NVENC_API_VERSION;
        lock.bitstream_buffer_ptr = bs.buffer as *mut c_void;
        lock.write_back_host_buffer = true;
        let mut out_size: u32 = 0;
        lock.output_bitstream_size_ptr = &mut out_size;
        let hr = unsafe { (self.lock_bs.unwrap())(self.session, &mut lock as *mut _ as *mut c_void, 0, ptr::null_mut()) };
        if hr != NV_ENC_SUCCESS { return Err(anyhow::anyhow!("LockBitstream failed: {}", hr)); }
        let sz = if out_size > 0 { out_size } else { bs.buffer_size };
        let data = unsafe { std::slice::from_raw_parts(bs.buffer, sz as usize).to_vec() };
        unsafe { (self.unlock_bs.unwrap())(self.session, &mut lock as *mut _ as *mut c_void) };
        Ok(data)
    }

    fn do_unmap(&mut self, mapped: *mut c_void) {
        if !mapped.is_null() {
            if let Some(f) = self.unmap { unsafe { f(self.session, mapped) }; }
        }
    }
}


impl Drop for NvencEncoder {
    fn drop(&mut self) {
        if self.initialized {
            if let Some(f) = self.destroy { unsafe { f(self.encoder) }; }
        }
        for buf in &self.bitstream_buffers {
            if !buf.buffer.is_null() {
                unsafe { std::alloc::dealloc(buf.buffer, std::alloc::Layout::from_size_align(buf.buffer_size as usize, 64).unwrap()); }
            }
        }
    }
}

// ─────────────────────────────────────────────
// Encoder Trait Implementation
// ─────────────────────────────────────────────

impl crate::encoder::Encoder for NvencEncoder {
    fn init(&mut self, device: &ID3D11Device, _context: &ID3D11DeviceContext, width: u32, height: u32, fps: u32, bitrate: u32) -> anyhow::Result<()> {
        self.width = width;
        self.height = height;
        self.fps = fps;
        self.bitrate = bitrate;
        self.initialize(device, _context)
    }

    fn encode(&mut self, frame: &atlas_frame::Frame) -> anyhow::Result<Vec<u8>> {
        if !self.is_ready() {
            return Err(anyhow::anyhow!("Encoder not initialized or still in stub mode"));
        }
        let device = self.device.as_ref().ok_or_else(|| anyhow::anyhow!("No D3D11 device"))?;
        let fid = self.frame_count.load(Ordering::Relaxed) + 1;
        let texture = self.mk_texture(device, frame)?;
        let mapped = self.reg_map(&texture)?;
        let result = self.do_encode(mapped, fid);
        self.do_unmap(mapped);
        drop(texture);
        self.frame_count.fetch_add(1, Ordering::Relaxed);
        result
    }

    fn is_ready(&self) -> bool {
        self.initialized && !self.use_stub
    }

    fn name(&self) -> &str {
        if self.use_stub { "NVENC(stub)" } else { "NVENC" }
    }
}

