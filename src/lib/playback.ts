import Hls from "hls.js";
import { streamProxyUrl } from "./format";

export interface StreamSource {
  videoUrl: string;
  hlsEnabled?: boolean;
  hlsMasterPlaylistUrl?: string | null;
}

/**
 * Attaches a video source to a <video> element with a robust strategy:
 *  - hls.js when MediaSource is available (Chrome/Firefox, WebKitGTK w/ MSE),
 *  - native HLS on Safari,
 *  - progressive MP4 as the universal fallback (also used if HLS fails, which
 *    is common on WebKitGTK).
 * Returns a cleanup function.
 */
export function attachStream(
  el: HTMLVideoElement,
  source: StreamSource,
  startAt = 0,
): () => void {
  const useHls = !!source.hlsEnabled && !!source.hlsMasterPlaylistUrl;
  const hlsSrc = streamProxyUrl(source.hlsMasterPlaylistUrl ?? "");
  const progressiveSrc = streamProxyUrl(source.videoUrl);

  let hls: Hls | null = null;

  const applyResume = () => {
    if (startAt <= 0) return;
    const onMeta = () => {
      el.currentTime = startAt;
      el.removeEventListener("loadedmetadata", onMeta);
    };
    el.addEventListener("loadedmetadata", onMeta);
  };

  const playProgressive = () => {
    hls?.destroy();
    hls = null;
    el.src = progressiveSrc;
    applyResume();
    void el.play?.().catch(() => {});
  };

  const nativeHlsOk = el.canPlayType("application/vnd.apple.mpegurl") !== "";
  const inTauri =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  if (inTauri && source.videoUrl) {
    // WebKitGTK (desktop): play the full-quality progressive MP4 natively via
    // GStreamer. hls.js/MSE here is stuttery and picks low bitrates, while the
    // GStreamer path is smooth (same as the hover previews) and full quality.
    playProgressive();
  } else if (useHls && Hls.isSupported()) {
    hls = new Hls({ enableWorker: true, startPosition: startAt || -1 });
    hls.on(Hls.Events.ERROR, (_evt, data) => {
      if (data.fatal) playProgressive();
    });
    hls.loadSource(hlsSrc);
    hls.attachMedia(el);
  } else if (useHls && nativeHlsOk) {
    el.src = hlsSrc;
    applyResume();
  } else {
    playProgressive();
  }

  const onError = () => {
    if (el.src !== progressiveSrc && progressiveSrc) playProgressive();
  };
  el.addEventListener("error", onError);

  return () => {
    el.removeEventListener("error", onError);
    hls?.destroy();
  };
}
