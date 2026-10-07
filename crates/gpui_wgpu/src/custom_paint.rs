//! Lets an application draw part of a frame with its own wgpu pipelines, for content GPUI's
//! primitives draw poorly, such as maps or 3D views.

use std::{any::Any, sync::Arc};

use gpui::{Bounds, DevicePixels, ScaledPixels, Size};

/// Draws a `gpui::PaintCustom` primitive. Paint one with
/// `window.paint_custom(bounds, custom_painter(painter))`.
pub trait CustomPaint: Send + Sync + 'static {
    /// Called for every frame the primitive is in, before the frame's main render pass begins:
    /// upload data, and record passes of its own into `cx.encoder`, such as drawing into a
    /// multisampled texture that `paint` then composites.
    fn prepare(&self, _cx: &mut CustomPrepareContext<'_>) {}

    /// Draws into the frame's main render pass, in the primitive's place in the paint order.
    /// The scissor rectangle is set to the primitive's clip. Output is blended as premultiplied
    /// alpha onto the frame's target, whose format is `cx.target_format`.
    fn paint(&self, pass: &mut wgpu::RenderPass<'_>, cx: &CustomPaintContext<'_>);
}

/// What `CustomPaint::prepare` gets.
pub struct CustomPrepareContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub target_format: wgpu::TextureFormat,
    /// The frame's size.
    pub viewport: Size<DevicePixels>,
    /// Where the primitive draws.
    pub bounds: Bounds<ScaledPixels>,
    /// The part of the frame it may draw on.
    pub clip: Bounds<ScaledPixels>,
}

/// What `CustomPaint::paint` gets.
pub struct CustomPaintContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub target_format: wgpu::TextureFormat,
    pub viewport: Size<DevicePixels>,
    pub bounds: Bounds<ScaledPixels>,
    pub clip: Bounds<ScaledPixels>,
}

/// Wraps a painter for `Window::paint_custom`.
pub fn custom_painter(painter: Arc<dyn CustomPaint>) -> Arc<dyn Any + Send + Sync> {
    Arc::new(painter)
}

pub(crate) fn painter(custom: &gpui::PaintCustom) -> Option<&Arc<dyn CustomPaint>> {
    custom.painter.downcast_ref::<Arc<dyn CustomPaint>>()
}

/// The clip in whole pixels inside the frame, as x, y, width and height; None when empty.
pub(crate) fn scissor(clip: Bounds<ScaledPixels>, viewport: Size<DevicePixels>) -> Option<[u32; 4]> {
    let (vw, vh) = (viewport.width.0.max(0) as f32, viewport.height.0.max(0) as f32);
    let x0 = clip.origin.x.0.floor().clamp(0.0, vw);
    let y0 = clip.origin.y.0.floor().clamp(0.0, vh);
    let x1 = (clip.origin.x.0 + clip.size.width.0).ceil().clamp(0.0, vw);
    let y1 = (clip.origin.y.0 + clip.size.height.0).ceil().clamp(0.0, vh);
    (x1 > x0 && y1 > y0).then(|| [x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32])
}
