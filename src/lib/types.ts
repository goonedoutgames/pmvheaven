// Shared types describing PMVHaven API objects and our normalized shapes.

export interface HlsVariant {
  resolution: string;
  width: number;
  height: number;
  bandwidth: number;
  playlistUrl: string;
}

export interface MusicTrack {
  artist: string;
  song: string;
}

export interface TimelineThumbnail {
  url: string;
  captureTime: number;
}

/** A lightweight video shape used in listings, grids and history rows. */
export interface VideoSummary {
  id: string;
  title: string;
  uploader: string;
  uploaderUsername: string;
  thumbnailUrl: string;
  previewUrl?: string;
  views: number;
  duration: string;
  durationSeconds: number;
  aspectRatio: number;
  likes: number;
  dislikes: number;
  rating: number;
  tags: string[];
  uploadDate: string;
  isRemix?: boolean;
  hasVoiceOver?: boolean;
  hasExtremeContent?: boolean;
}

/** The full detail shape used on the watch page. */
export interface VideoDetail extends VideoSummary {
  description: string;
  videoUrl: string;
  hlsMasterPlaylistUrl?: string;
  hlsEnabled: boolean;
  hlsVariants: HlsVariant[];
  width: number;
  height: number;
  favorites: number;
  creator: string[];
  stars: string[];
  music: MusicTrack[];
  timelineThumbnails: TimelineThumbnail[];
  uploaderAvatarUrl?: string;
  uploaderId?: string;
  // User-specific (only populated when authenticated)
  watchProgress?: number;
  isLiked?: boolean;
  isDisliked?: boolean;
  isFavorited?: boolean;
  isWatchLater?: boolean;
}

export interface Pagination {
  page: number;
  limit: number;
  total: number;
  totalPages: number;
  hasNext: boolean;
  hasPrev: boolean;
}

export interface Paged<T> {
  items: T[];
  pagination: Pagination;
}

export type VideoSort =
  | "-uploadDate"
  | "uploadDate"
  | "-views"
  | "views"
  | "-likes"
  | "-bayesianRating";

export interface FeedParams {
  page?: number;
  limit?: number;
  sort?: VideoSort;
  tags?: string;
  creator?: string;
  uploader?: string;
}

export interface HistoryEntry {
  video: VideoSummary;
  watchedAt: string;
  progress: number;
}

export interface PopularTag {
  name: string;
  usageCount: number;
}

export interface AccountUser {
  id: string;
  username: string;
  email?: string;
  avatarUrl?: string;
}
