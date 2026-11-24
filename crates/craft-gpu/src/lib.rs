// From EffectCraft (https://github.com/storytold/effectcraft, commit 1738360), MIT OR Apache-2.0.
// Copied into Beethoven Craft on 2026-11-23 and trimmed: no Renderer, 3D, particles simulation, fill or tests.
// Added for Beethoven Craft: `mesa`, `bc_lista` and `shaders/bc.wgsl`.
//! The EffectCraft GPU compositor (Mercury GPU Acceleration's counterpart), on wgpu compute
//! shaders: Metal, Vulkan, Direct3D 12 and WebGPU.
//!
//! The CPU [`Renderer`] stays the reference and keeps rendering layer *content* (sources, masks,
//! CPU effects, layer styles) into its layer cache. [`Gpu`] composites: it uploads the cached
//! layer buffers (once per buffer), applies the layer transforms with the CPU's sampling
//! (nearest / bilinear / bicubic, minification pre-filter), motion-blur sub-samples, track
//! mattes, Preserve Transparency, layer styles' passes, all 38 blend modes and the 8/16 bpc
//! clamping and quantisation, and converts colour spaces. Classic 3D runs composite here too
//! (`classic3d`: per-pixel fragment sort, lights, ray-cast shadows), and adjustment layers run
//! their effect stacks on the GPU-resident comp. Wireframe-quality outlines are drawn here too.
//!
//! Advanced 3D comps rasterise on a render pipeline (`advanced3d.wgsl`: depth buffer, PBR,
//! image-based light, shadow maps) and finish in compute kernels (`adv3d.wgsl`: supersampling
//! resolve, motion-blur sub-samples, iris depth of field, encoding, compositing), see
//! [`Accelerator::render_3d`] and `Renderer::prepare_adv_run`. Runs with layers on the 2D
//! compositing path (blend modes, track mattes, Preserve Transparency:
//! `Renderer::split_adv_run`) render each such layer alone, hide it behind the nearer main
//! scene and composite it through the 2D path here too; environment backgrounds (Classic and
//! Advanced 3D) draw their sky in a kernel (`Renderer::sky_draw`).
//!
//! GPU effects ([`effectcraft_effects::GPU_EFFECTS`]) run as compute kernels with the CPU
//! effect's exact steps (padding, box-blur radii, parameter conversions); chains of them are
//! uploaded and read back once. Each family lives in its own module with its own WGSL file
//! (`fx_color`, `fx_depth`, `fx_distort`, `fx_extra`, `fx_gen2`, `fx_generate`, `fx_key`,
//! `fx_light`, `fx_lut`, `fx_noise`, `fx_particles`, `fx_pixel2`, `fx_sim`, `fx_stylize`,
//! `fx_text`, `fx_time`, `fx_tone`, `fx_transition`, `fx_vr`, `fx_warp`); settings a kernel
//! cannot match fall back to the CPU (`catalog::gpu_supported`, or `None` from the family's
//! `apply`).
//! The 3D Channel effects read the layer's aux channels as an extra texture (`fx_depth`), the
//! colour-management effects interpret colour programs built next to the CPU effects
//! (`fx_lut`), simulations keep their state on the CPU and rasterise its per-frame plan here
//! (`fx_sim`, `fx_particles`), and so do the geometry generators (`fx_gen2`: bolts, strokes,
//! glyphs, audio marks, waves).
//!
//! Working textures come from a pool (`context::Pool`): a frame allocates several full-frame
//! RGBA f32 images per layer, and creating and zeroing those cost more than compositing a small
//! layer. A released texture is reused once every encoder that could still read it has been
//! submitted. 8/16 bpc quantisation after each layer is fused into that layer's composite.
//! [`Backend::Auto`](effectcraft_render::Backend) renders each comp on whichever compositor
//! measured faster for it ([`effectcraft_render::AutoPick`], kept by [`Gpu`]): a light comp
//! that the CPU composites in a millisecond stays there rather than paying for a full-frame
//! readback.
//!
//! GPU particles (`particles`): the stepped particle effects hand their simulation to
//! [`effectcraft_effects::psim::ParticleSim`], implemented here with one invocation per particle
//! and GPU-resident checkpoints.
//!
//! Plug a [`Gpu`] into [`Renderer::accel`] (it implements [`Accelerator`]); renders then use it
//! when [`RenderOpts::backend`](effectcraft_render::RenderOpts) asks for it. The viewer can
//! skip readback entirely with [`Gpu::render_display`], which leaves an RGBA8 texture for
//! egui-wgpu to draw.

// Es un recorte de EffectCraft: quedan piezas sin usar.
#![allow(dead_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod context;
pub mod deferred;
mod diagnostics;
mod effects;
mod fx_color;
mod fx_distort;
mod fx_extra;
mod fx_gen2;
mod fx_generate;
mod fx_key;
mod fx_light;
mod fx_lut;
mod fx_noise;
mod fx_particles;
mod fx_pixel2;
mod fx_sim;
mod fx_stylize;
mod fx_text;
mod fx_time;
mod fx_tone;
mod fx_transition;
mod fx_vr;
mod fx_warp;
mod bc_lista;
mod memcap;
mod mesa;
mod ops;
mod readback;
mod upload;
mod yuv;

use std::sync::Arc;

pub use mesa::{Capa, Datos, Formato, Mesa};
pub use context::{GpuContext, GpuImage, TransferStats};
pub use diagnostics::{ScopedError, ScopedErrorObserver};
pub use wgpu;

use crate::context::Enc;

/// One effect of a GPU effect chain (from `effectcraft-render`).
pub struct FxStep<'x> {
    pub spec: &'static effectcraft_effects::EffectSpec,
    pub ctx: effectcraft_effects::EffectCtx<'x>,
}

/// What exports size their frames in flight by (from `effectcraft-render`).
pub struct FrameMemory {
    pub budget: u64,
    pub peak: u64,
    pub frame: u64,
    pub reserve: u64,
}


/// Why [`Gpu::within_memory`] discarded a frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// The GPU context is retired (device lost, a readback timed out, an uncaptured error).
    Retired(String),
    /// Out of video memory (or a driver reporting a failed allocation as an internal error),
    /// with any validation errors it went on to cause.
    Memory(String),
    /// The device rejected a command with no allocation failure before it (an invalid texture
    /// or parameters, #1319): a compositor bug, not memory pressure.
    Validation(String),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Retired(e) | FrameError::Memory(e) | FrameError::Validation(e) => f.write_str(e),
        }
    }
}

impl std::error::Error for FrameError {}

/// The GPU compositor (cheap to clone; one device shared by all clones).
#[derive(Clone)]
pub struct Gpu {
    scoped_observations: Arc<diagnostics::ScopedObservations>,
    ctx: Arc<GpuContext>,
}

/// A viewer frame left on the GPU: premultiplied RGBA8 (`wgpu::TextureFormat::Rgba8Unorm`) with
/// a full mip chain (for minified views), ready for
/// `egui_wgpu::Renderer::register_native_texture_with_sampler_options`.
#[derive(Clone, Debug)]
pub struct DisplayFrame {
    pub texture: wgpu::Texture,
    pub width: u32,
    pub height: u32,
}

impl Gpu {
    /// On a device of its own (CLI, tests, benchmarks); `None` without a usable adapter.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn headless() -> Option<Gpu> {
        Gpu::try_headless().map_err(|e| log::info!("gpu: {e}")).ok()
    }

    /// [`Gpu::headless`], with why there is none.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_headless() -> Result<Gpu, String> {
        pollster::block_on(GpuContext::request()).map(Gpu::from_context)
    }

    /// On an existing device (the desktop app shares egui-wgpu's).
    pub fn new(adapter: &wgpu::Adapter, device: wgpu::Device, queue: wgpu::Queue) -> Result<Gpu, String> {
        GpuContext::new(adapter, device, queue).map(Gpu::from_context)
    }

    pub fn from_context(ctx: GpuContext) -> Gpu {
        Gpu { ctx: Arc::new(ctx), scoped_observations: Default::default() }
    }

    /// A device of its own without blocking, with deferred readbacks (a browser worker, see
    /// [`deferred`]); natively the same device works too (tests). `Err` says why there is none.
    pub async fn request_deferred() -> Result<Gpu, String> {
        let mut ctx = GpuContext::request().await?;
        ctx.set_deferred(true);
        Ok(Gpu::from_context(ctx))
    }

    /// Install a host's fixed-category observer before rendering. It is called at most once
    /// per category across all clones, outside device/renderer locks. Returns false if already
    /// registered. The observer must remain brief and must not perform GPU work.
    pub fn set_scoped_error_observer(&self, observer: ScopedErrorObserver) -> bool {
        self.scoped_observations.install(observer)
    }

    /// [`Gpu::request_deferred`] blocking (tests of the browser worker's path, natively).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn headless_deferred() -> Option<Gpu> {
        pollster::block_on(Gpu::request_deferred()).map_err(|e| log::info!("gpu: {e}")).ok()
    }

    /// The deferred readback state, when readbacks are deferred.
    pub fn deferred(&self) -> Option<&Arc<deferred::Deferred>> {
        self.ctx.deferred()
    }

    /// Resolves once no deferred readback is in flight (immediately without deferred readbacks).
    pub async fn settled(&self) {
        if let Some(d) = self.ctx.deferred() {
            d.settled().await;
        }
    }

    /// Deliver finished readbacks without blocking (native; the browser delivers them itself).
    pub fn poll(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        let _ = self.ctx.poll_readbacks(wgpu::PollType::Poll);
    }

    /// `{passes, readbacks, inFlight}` of deferred rendering (diagnostics).
    pub fn deferred_stats(&self) -> Option<(u64, u64, usize)> {
        let d = self.ctx.deferred()?;
        Some((d.passes.load(std::sync::atomic::Ordering::Relaxed), d.readbacks.load(std::sync::atomic::Ordering::Relaxed), d.in_flight()))
    }

    pub fn context(&self) -> &GpuContext {
        &self.ctx
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.ctx.device
    }

    /// Read a display frame back as premultiplied RGBA8 (Info panel sampling, eyedroppers).
    pub fn read_display(&self, f: &DisplayFrame) -> Option<Vec<u8>> {
        Enc::new(&self.ctx).read_texture(&f.texture, f.width, f.height, 4)
    }

    /// [`Gpu::read_display`] without blocking (the browser's main thread): `done` gets the bytes
    /// once the GPU has them.
    pub fn read_display_async(&self, f: &DisplayFrame, done: impl FnOnce(Option<Vec<u8>>) + wgpu::WasmNotSend + 'static) {
        if let Err(e) = self.ctx.check_health() {
            log::error!("gpu display readback: {e}");
            crate::readback::reject(done, e);
            return;
        }
        Enc::new(&self.ctx).read_texture_async(&f.texture, f.width, f.height, 4, done);
    }

    /// Run work in native thread-local out-of-memory, internal-error and validation scopes. A
    /// failed result is discarded and the textures and buffers kept for reuse are released;
    /// callers can retry with a fresh CPU renderer. Some drivers report a failed allocation as an
    /// internal error rather than out-of-memory (Adreno's `ERROR_INITIALIZATION_FAILED`, #591),
    /// so both count. A failed allocation leaves an invalid texture whose use is then a
    /// validation error (`Texture::create_view`, #1319): caught here, it fails this frame only
    /// instead of retiring the GPU compositor for the session as an uncaptured error.
    /// Browser callers use the host/owned-device failure bridge and bounded readbacks instead
    /// of blocking their event loop on an error-scope future.
    pub fn within_memory<T>(&self, f: impl FnOnce() -> T) -> Result<T, FrameError> {
        if let Err(error) = self.ctx.check_health() {
            // Queued jobs may still own this context after the UI detached it. Release its
            // working caches before CPU fallback adds host-memory pressure (#3357).
            self.ctx.discard_kept();
            return Err(FrameError::Retired(error));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let oom = self.ctx.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
            let internal = self.ctx.device.push_error_scope(wgpu::ErrorFilter::Internal);
            let validation = self.ctx.device.push_error_scope(wgpu::ErrorFilter::Validation);
            let out = f();
            let validation = pollster::block_on(validation.pop());
            let internal = pollster::block_on(internal.pop());
            let oom = pollster::block_on(oom.pop());
            // Preserve the typed scope observations before FrameError::Memory combines OOM
            // and internal errors. An internal error alone is not evidence of actual OOM.
            self.scoped_observations.report_scopes(oom.is_some(), internal.is_some(), validation.is_some());
            let memory = oom.is_some() || internal.is_some();
            // The allocation failure first: a validation error is often its consequence.
            let errors: Vec<String> = [oom, internal, validation].into_iter().flatten().map(|e| e.to_string()).collect();
            if errors.is_empty() {
                return self.finish_memory_attempt(out);
            }
            // Its textures go back to the pool when dropped: drop them before emptying it.
            drop(out);
            self.ctx.discard_kept();
            let errors = errors.join("; then: ");
            Err(if memory { FrameError::Memory(errors) } else { FrameError::Validation(errors) })
        }
        #[cfg(target_arch = "wasm32")]
        {
            let out = f();
            self.finish_memory_attempt(out)
        }
    }

    /// Retirement without a scoped error (for example, a readback timeout) must release
    /// the same retained resources as a rejected allocation before the caller retries.
    fn finish_memory_attempt<T>(&self, out: T) -> Result<T, FrameError> {
        match self.ctx.check_health() {
            Ok(()) => Ok(out),
            Err(error) => {
                // Dropping the result can return leased textures to the pool.
                drop(out);
                self.ctx.discard_kept();
                Err(FrameError::Retired(error))
            }
        }
    }

    /// Wait for submitted GPU work with the readback deadline (benchmarks).
    pub fn wait(&self) {
        let _ = self.ctx.poll_readbacks(wgpu::PollType::Wait { submission_index: None, timeout: Some(crate::readback::TIMEOUT) });
    }
}

