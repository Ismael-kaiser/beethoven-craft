//! Export frames converted on the device to the H.264 encoder's input (8-bit BT.709 limited
//! range 4:2:0, see `shaders/yuv.wgsl`): a 4K frame reads back 12 MB of planes instead of
//! 133 MB of RGBA f32, and the CPU skips `to_rgba8_over` + `rgba_to_yuv420`.

use wgpu::util::DeviceExt;

use crate::context::{Enc, GpuImage};

pub(crate) struct YuvPipe {
    bgl: wgpu::BindGroupLayout,
    pipe: wgpu::ComputePipeline,
}

impl YuvPipe {
    pub(crate) fn new(device: &wgpu::Device) -> Result<YuvPipe, String> {
        crate::context::init_resource(device, "yuv420", || {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("yuv420"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/yuv.wgsl").into()),
            });
            let storage =
                wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None };
            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("yuv420"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: storage, count: None },
                ],
            });
            let layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("yuv420"), bind_group_layouts: &[Some(&bgl)], immediate_size: 0 });
            let pipe = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("yuv420"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some("yuv420"),
                compilation_options: Default::default(),
                cache: None,
            });
            YuvPipe { bgl, pipe }
        })
    }
}

/// The frame sizes the kernel handles: width a multiple of 8 (4 chroma samples per
/// invocation), height even.
pub(crate) fn supported(w: u32, h: u32) -> bool {
    w > 0 && h > 0 && w.is_multiple_of(8) && h.is_multiple_of(2)
}

/// Bytes of the planes: Y `w×h`, then U and V `w/2 × h/2`.
pub(crate) fn planes_len(w: u32, h: u32) -> u64 {
    w as u64 * h as u64 + 2 * ((w / 2) as u64 * (h / 2) as u64)
}

/// Whether the [`planes_len`] storage and staging buffers for `w×h` fit within `limits`.
pub(crate) fn fits_limits(w: u32, h: u32, limits: &wgpu::Limits) -> bool {
    let size = planes_len(w, h);
    size <= limits.max_storage_buffer_binding_size && size <= limits.max_buffer_size
}

/// Record the conversion of `img` over `bg` into a new `MAP_READ` buffer of
/// [`planes_len`] bytes and submit it. `None`: the size is not [`supported`], exceeds the
/// device's buffer limits, or buffer allocation failed.
pub(crate) fn record(e: &mut Enc, yp: &YuvPipe, img: &GpuImage, bg: [f32; 3]) -> Option<(wgpu::Buffer, wgpu::Buffer)> {
    let (w, h) = (img.width, img.height);
    let g = e.g;
    if !supported(w, h) || !fits_limits(w, h, &g.device.limits()) {
        return None;
    }
    let cw = w / 2;
    let size = planes_len(w, h);
    let mut params = [0u8; 32];
    // w * h fits in u32: both are at most the texture limit (16384).
    for (i, v) in [w, h, cw, w * h / 4].into_iter().enumerate() {
        params[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    for (i, v) in [bg[0], bg[1], bg[2], 1.0].into_iter().enumerate() {
        params[16 + i * 4..16 + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    // The buffers of a finished frame are used again: creating and zeroing them costs more than the kernel.
    let kept = g.yuv_buffers.lock().ok().and_then(|mut v| v.iter().position(|b| b.0.size() == size).map(|i| v.swap_remove(i)));
    let (out, stg, ub) = match crate::context::init_resource(&g.device, "yuv420 buffers", || {
        let (out, stg) = kept.unwrap_or_else(|| {
        let out = g.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("yuv420 out"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let stg = g.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("yuv420 staging"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        (out, stg)
        });
        let ub = g.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("yuv420 params"),
            contents: &params,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        (out, stg, ub)
    }) {
        Ok(buffers) => buffers,
        Err(err) => {
            log::warn!(target: "effectcraft_gpu", "yuv420 buffers: {err}");
            return None;
        }
    };
    let view = img.texture.create_view(&Default::default());
    let group = g.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("yuv420"),
        layout: &yp.bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
            wgpu::BindGroupEntry { binding: 2, resource: out.as_entire_binding() },
        ],
    });
    {
        let enc = e.record(&[img]);
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("yuv420"), timestamp_writes: None });
            pass.set_pipeline(&yp.pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups((w / 8).div_ceil(8), (h / 2).div_ceil(8), 1);
        }
        enc.copy_buffer_to_buffer(&out, 0, &stg, 0, size);
    }
    e.submit();
    Some((out, stg))
}

/// Convert `img` over `bg` and read the planes back, waiting for the device.
pub(crate) fn convert(e: &mut Enc, yp: &YuvPipe, img: &GpuImage, bg: [f32; 3]) -> Option<Vec<u8>> {
    let (out, stg) = record(e, yp, img, bg)?;
    let n = planes_len(img.width, img.height) as usize;
    let v = e.g.read_buffer_after(&stg, e.last_submission(), n, n, 1).map_err(|err| log::error!(target: "effectcraft_gpu", "yuv420 readback: {err}")).ok()?;
    if let Ok(mut b) = e.g.yuv_buffers.lock() {
        // Those of other sizes are from an earlier export.
        b.retain(|x| x.0.size() == out.size());
        if b.len() < 4 {
            b.push((out, stg));
        }
    }
    Some(v)
}

