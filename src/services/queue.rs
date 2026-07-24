use crate::models::VideoSummary;
use crate::paths;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueState {
    pub items: Vec<VideoSummary>,
}

static QUEUE: Mutex<QueueState> = Mutex::new(QueueState { items: Vec::new() });

pub fn load_queue() {
    let path = paths::queue_path();
    if let Ok(data) = std::fs::read_to_string(path) {
        if let Ok(state) = serde_json::from_str::<QueueState>(&data) {
            *QUEUE.lock().unwrap() = state;
        }
    }
}

fn persist(state: &QueueState) {
    let _ = paths::ensure_data_dir();
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(paths::queue_path(), json);
    }
}

pub fn snapshot() -> QueueState {
    QUEUE.lock().unwrap().clone()
}

pub fn add(video: VideoSummary) {
    let mut q = QUEUE.lock().unwrap();
    if q.items.iter().any(|v| v.id == video.id) {
        return;
    }
    q.items.push(video);
    persist(&q);
}

pub fn play_next(video: VideoSummary) {
    let mut q = QUEUE.lock().unwrap();
    q.items.retain(|v| v.id != video.id);
    q.items.insert(0, video);
    persist(&q);
}

pub fn remove(id: &str) {
    let mut q = QUEUE.lock().unwrap();
    q.items.retain(|v| v.id != id);
    persist(&q);
}

pub fn move_item(from: usize, to: usize) {
    let mut q = QUEUE.lock().unwrap();
    if from >= q.items.len() || to >= q.items.len() || from == to {
        return;
    }
    let item = q.items.remove(from);
    q.items.insert(to, item);
    persist(&q);
}

pub fn clear() {
    let mut q = QUEUE.lock().unwrap();
    q.items.clear();
    persist(&q);
}

pub fn shift() -> Option<VideoSummary> {
    let mut q = QUEUE.lock().unwrap();
    if q.items.is_empty() {
        return None;
    }
    let item = q.items.remove(0);
    persist(&q);
    Some(item)
}

/// Remove a single item by id and return it (rest of the queue stays intact).
pub fn take(id: &str) -> Option<VideoSummary> {
    let mut q = QUEUE.lock().unwrap();
    let pos = q.items.iter().position(|v| v.id == id)?;
    let item = q.items.remove(pos);
    persist(&q);
    Some(item)
}

pub fn peek() -> Option<VideoSummary> {
    QUEUE.lock().unwrap().items.first().cloned()
}

pub fn is_queued(id: &str) -> bool {
    QUEUE.lock().unwrap().items.iter().any(|v| v.id == id)
}

pub fn len() -> usize {
    QUEUE.lock().unwrap().items.len()
}
