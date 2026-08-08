//! Typed app contexts — Dioxus keys context by **type**, so multiple
//! bare `Signal<bool>` providers silently overwrite each other.

use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct QueueOpen(pub Signal<bool>);

#[derive(Clone, Copy)]
pub struct PlayerFs(pub Signal<bool>);

#[derive(Clone, Copy)]
pub struct HoverPreviews(pub Signal<bool>);

#[derive(Clone, Copy)]
pub struct PausePreviewsWhilePlaying(pub Signal<bool>);

/// Hover preview volume in `0.0..=1.0` (0 = muted).
#[derive(Clone, Copy)]
pub struct HoverPreviewVolume(pub Signal<f32>);

#[derive(Clone, Copy)]
pub struct QueueTick(pub Signal<u32>);

#[derive(Clone, Copy)]
pub struct StartAt(pub Signal<f64>);

#[derive(Clone, Copy)]
pub struct ProxyBase(pub Signal<String>);

#[derive(Clone, Copy)]
pub struct PlayerRailW(pub Signal<Option<u32>>);

#[derive(Clone, Copy)]
pub struct PlayerQueueH(pub Signal<Option<u32>>);
