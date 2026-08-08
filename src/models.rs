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

/// Streamable payload for the persistent player rail (survives route changes).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PlayableVideo {
    pub summary: VideoSummary,
    pub video_url: String,
    pub hls_master_playlist_url: Option<String>,
    pub hls_enabled: bool,
    #[serde(default)]
    pub hls_variants: Vec<HlsVariant>,
    pub watch_progress: f64,
}

impl From<&VideoDetail> for PlayableVideo {
    fn from(d: &VideoDetail) -> Self {
        let mut summary = d.summary.clone();
        // Prefer real frame size when the API reports it (vertical clips often
        // ship a wrong/default aspectRatio on the summary).
        if d.width > 0 && d.height > 0 {
            summary.aspect_ratio = d.width as f64 / d.height as f64;
        } else if summary.aspect_ratio <= 0.0 {
            summary.aspect_ratio = 16.0 / 9.0;
        }
        Self {
            summary,
            video_url: d.video_url.clone(),
            hls_master_playlist_url: d.hls_master_playlist_url.clone(),
            hls_enabled: d.hls_enabled,
            hls_variants: d.hls_variants.clone(),
            watch_progress: d.watch_progress,
        }
    }
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
            // Site default is -releaseDate (same order as upload for most clips).
            Self::Newest => "-releaseDate",
            Self::Oldest => "releaseDate",
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
            Self::TopRated => "Most popular",
        }
    }

    pub fn from_api(s: &str) -> Self {
        match s {
            "releaseDate" | "uploadDate" => Self::Oldest,
            "-views" => Self::MostViews,
            "views" => Self::LeastViews,
            "-likes" => Self::MostLiked,
            "-bayesianRating" => Self::TopRated,
            _ => Self::Newest,
        }
    }
}

/// Tri-state content chip: off → include → exclude → off (matches site).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ContentChip {
    #[default]
    Off,
    Include,
    Exclude,
}

impl ContentChip {
    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::Include,
            Self::Include => Self::Exclude,
            Self::Exclude => Self::Off,
        }
    }
}

/// Full browse/search filter set mirroring pmvhaven.com dashboard filters.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FeedParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub sort: Option<VideoSort>,
    /// Comma-separated tags.
    pub tags: Option<String>,
    pub exclude_tags: Option<String>,
    /// Stars / models (comma-separated).
    pub stars: Option<String>,
    pub exclude_stars: Option<String>,
    pub music_tags: Option<String>,
    pub music_artist: Option<String>,
    pub music_song: Option<String>,
    pub creators: Option<String>,
    pub exclude_creators: Option<String>,
    pub creator: Option<String>,
    pub uploader: Option<String>,
    pub featured_model: Option<String>,
    /// "OR" | "AND"
    pub tag_mode: Option<String>,
    pub expand_tags: Option<bool>,
    pub upload_date_from: Option<String>,
    pub upload_date_to: Option<String>,
    /// Duration bounds in **seconds**.
    pub duration_min: Option<u32>,
    pub duration_max: Option<u32>,
    pub min_rating: Option<u32>,
    pub max_rating: Option<u32>,
    pub min_views: Option<u64>,
    pub max_views: Option<u64>,
    pub min_height: Option<u32>,
    pub max_height: Option<u32>,
    /// Comma-separated aspects e.g. "16:9,9:16".
    pub aspect: Option<String>,
    pub include_gay: bool,
    pub exclude_gay: bool,
    pub include_trans: bool,
    pub exclude_trans: bool,
    pub include_voice_over: bool,
    pub exclude_voice_over: bool,
    pub include_remix: bool,
    pub exclude_remix: bool,
    pub include_extreme: bool,
    pub exclude_extreme: bool,
    pub include_promotional: bool,
    pub exclude_promotional: bool,
    pub include_explicit: bool,
    pub exclude_explicit: bool,
    pub non_nude_only: bool,
    pub watched_only: bool,
    pub exclude_watched: bool,
    pub funscript_only: bool,
    pub exclude_funscript: bool,
    pub subscribed_only: bool,
    pub favorites_only: bool,
    pub trending: bool,
    pub personalized_only: bool,
}

impl FeedParams {
    /// Append non-default filters as query pairs for `/api/videos`.
    pub fn to_query(&self) -> Vec<(String, String)> {
        let mut q = vec![
            ("page".into(), self.page.unwrap_or(1).to_string()),
            ("limit".into(), self.limit.unwrap_or(32).to_string()),
        ];
        let sort = self.sort.unwrap_or(VideoSort::Newest).as_api();
        q.push(("sort".into(), sort.to_string()));

        let push = |q: &mut Vec<(String, String)>, k: &str, v: &Option<String>| {
            if let Some(s) = v {
                if !s.is_empty() {
                    q.push((k.into(), s.clone()));
                }
            }
        };
        push(&mut q, "tags", &self.tags);
        push(&mut q, "excludeTags", &self.exclude_tags);
        push(&mut q, "stars", &self.stars);
        push(&mut q, "excludeStars", &self.exclude_stars);
        push(&mut q, "musicTags", &self.music_tags);
        push(&mut q, "musicArtist", &self.music_artist);
        push(&mut q, "musicSong", &self.music_song);
        push(&mut q, "creators", &self.creators);
        push(&mut q, "excludeCreators", &self.exclude_creators);
        push(&mut q, "creator", &self.creator);
        push(&mut q, "uploader", &self.uploader);
        push(&mut q, "featuredModel", &self.featured_model);
        push(&mut q, "aspect", &self.aspect);
        push(&mut q, "uploadDateFrom", &self.upload_date_from);
        push(&mut q, "uploadDateTo", &self.upload_date_to);

        let mode = self.tag_mode.as_deref().unwrap_or("OR");
        q.push(("tagMode".into(), mode.to_string()));
        let expand = if mode == "AND" {
            false
        } else {
            self.expand_tags.unwrap_or(false)
        };
        q.push(("expandTags".into(), if expand { "true" } else { "false" }.into()));

        if let Some(v) = self.duration_min {
            q.push(("durationMin".into(), v.to_string()));
        }
        if let Some(v) = self.duration_max {
            q.push(("durationMax".into(), v.to_string()));
        }
        if let Some(v) = self.min_rating {
            q.push(("minRating".into(), v.to_string()));
        }
        if let Some(v) = self.max_rating {
            q.push(("maxRating".into(), v.to_string()));
        }
        if let Some(v) = self.min_views {
            q.push(("minViews".into(), v.to_string()));
        }
        if let Some(v) = self.max_views {
            q.push(("maxViews".into(), v.to_string()));
        }
        if let Some(v) = self.min_height {
            q.push(("minHeight".into(), v.to_string()));
        }
        if let Some(v) = self.max_height {
            q.push(("maxHeight".into(), v.to_string()));
        }

        let flag = |q: &mut Vec<(String, String)>, on: bool, key: &str| {
            if on {
                q.push((key.into(), "true".into()));
            }
        };
        flag(&mut q, self.include_gay, "includeGay");
        flag(&mut q, self.exclude_gay, "excludeGay");
        flag(&mut q, self.include_trans, "includeTrans");
        flag(&mut q, self.exclude_trans, "excludeTrans");
        flag(&mut q, self.include_voice_over, "includeVoiceOver");
        flag(&mut q, self.exclude_voice_over, "excludeVoiceOver");
        flag(&mut q, self.include_remix, "includeRemix");
        flag(&mut q, self.exclude_remix, "excludeRemix");
        flag(&mut q, self.include_extreme, "includeExtreme");
        flag(&mut q, self.exclude_extreme, "excludeExtreme");
        flag(&mut q, self.include_promotional, "includePromotional");
        flag(&mut q, self.exclude_promotional, "excludePromotional");
        flag(&mut q, self.include_explicit, "includeExplicit");
        flag(&mut q, self.exclude_explicit, "excludeExplicit");
        flag(&mut q, self.non_nude_only, "nonNudeOnly");
        flag(&mut q, self.watched_only, "watchedOnly");
        flag(&mut q, self.exclude_watched, "excludeWatched");
        flag(&mut q, self.funscript_only, "funscriptOnly");
        flag(&mut q, self.exclude_funscript, "excludeFunscript");
        flag(&mut q, self.subscribed_only, "subscribedOnly");
        flag(&mut q, self.favorites_only, "favoritesOnly");
        flag(&mut q, self.trending, "trending");
        flag(&mut q, self.personalized_only, "personalizedOnly");
        q
    }
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
