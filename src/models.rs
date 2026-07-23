use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HlsVariant {
    pub resolution: String,
    pub width: u32,
    pub height: u32,
    pub bandwidth: u32,
    pub playlist_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MusicTrack {
    pub artist: String,
    pub song: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineThumbnail {
    pub url: String,
    pub capture_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct VideoSummary {
    pub id: String,
    pub title: String,
    pub uploader: String,
    pub uploader_username: String,
    pub thumbnail_url: String,
    pub preview_url: Option<String>,
    pub views: u64,
    pub duration: String,
    pub duration_seconds: u32,
    pub aspect_ratio: f64,
    pub likes: u64,
    pub dislikes: u64,
    pub rating: f64,
    pub tags: Vec<String>,
    pub upload_date: String,
    pub is_remix: bool,
    pub has_voice_over: bool,
    pub has_extreme_content: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct VideoDetail {
    #[serde(flatten)]
    pub summary: VideoSummary,
    pub description: String,
    pub video_url: String,
    pub hls_master_playlist_url: Option<String>,
    pub hls_enabled: bool,
    pub hls_variants: Vec<HlsVariant>,
    pub width: u32,
    pub height: u32,
    pub favorites: u64,
    pub creator: Vec<String>,
    pub stars: Vec<String>,
    pub music: Vec<MusicTrack>,
    pub timeline_thumbnails: Vec<TimelineThumbnail>,
    pub uploader_avatar_url: Option<String>,
    pub uploader_id: Option<String>,
    pub watch_progress: f64,
    pub is_liked: bool,
    pub is_disliked: bool,
    pub is_favorited: bool,
    pub is_watch_later: bool,
}

impl VideoDetail {
    pub fn id(&self) -> &str {
        &self.summary.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pagination {
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Paged<T> {
    pub items: Vec<T>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoSort {
    Newest,
    Oldest,
    MostViews,
    LeastViews,
    MostLiked,
    TopRated,
}

impl VideoSort {
    pub fn as_api(self) -> &'static str {
        match self {
            Self::Newest => "-uploadDate",
            Self::Oldest => "uploadDate",
            Self::MostViews => "-views",
            Self::LeastViews => "views",
            Self::MostLiked => "-likes",
            Self::TopRated => "-bayesianRating",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Newest => "Newest",
            Self::Oldest => "Oldest",
            Self::MostViews => "Most viewed",
            Self::LeastViews => "Least viewed",
            Self::MostLiked => "Most liked",
            Self::TopRated => "Top rated",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FeedParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub sort: Option<VideoSort>,
    pub tags: Option<String>,
    pub creator: Option<String>,
    pub uploader: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoryEntry {
    pub video: VideoSummary,
    pub watched_at: String,
    pub progress: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PopularTag {
    pub name: String,
    pub usage_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountUser {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteHistoryEntry {
    pub video_id: String,
    pub watched_at: String,
    pub progress: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteHistory {
    pub total_retained: u64,
    pub entries: Vec<RemoteHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncProgress {
    pub phase: String,
    pub processed: u64,
    pub total: u64,
    pub new_count: u64,
    pub total_retained: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub status: String,
    pub new_count: u64,
    pub seen_count: u64,
    pub total_retained: u64,
    pub message: Option<String>,
}
