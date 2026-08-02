//! Browse filter panel — mirrors pmvhaven.com dashboard filters.

use crate::models::{ContentChip, FeedParams, VideoSort};
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct BrowseFilterState {
    pub sort: VideoSort,
    pub open: bool,
    pub tags: String,
    pub exclude_tags: String,
    pub exclude_mode: bool,
    pub stars: String,
    pub music: String,
    pub creator: String,
    pub tag_mode_and: bool,
    pub expand_tags: bool,
    /// Preset: "", "today", "7days", "30days", "365days"
    pub uploaded: String,
    pub duration_min_min: Option<u32>,
    pub duration_max_min: Option<u32>,
    pub rating_min: Option<u32>,
    pub views_min: Option<u64>,
    pub views_max: Option<u64>,
    /// Quality keys: SD HD FHD QHD 4K
    pub qualities: Vec<&'static str>,
    pub aspects: Vec<&'static str>,
    pub gay: ContentChip,
    pub trans: ContentChip,
    pub voice: ContentChip,
    pub remix: ContentChip,
    pub extreme: ContentChip,
    pub promo: ContentChip,
    pub sfw: ContentChip,
    pub watched: ContentChip,
    pub funscript: ContentChip,
    pub subscriptions: ContentChip,
    pub favorites: ContentChip,
}

impl Default for BrowseFilterState {
    fn default() -> Self {
        Self {
            sort: VideoSort::Newest,
            open: true,
            tags: String::new(),
            exclude_tags: String::new(),
            exclude_mode: false,
            stars: String::new(),
            music: String::new(),
            creator: String::new(),
            tag_mode_and: false,
            expand_tags: false,
            uploaded: String::new(),
            duration_min_min: None,
            duration_max_min: None,
            rating_min: None,
            views_min: None,
            views_max: None,
            qualities: Vec::new(),
            aspects: Vec::new(),
            gay: ContentChip::Off,
            trans: ContentChip::Off,
            voice: ContentChip::Off,
            remix: ContentChip::Off,
            extreme: ContentChip::Off,
            promo: ContentChip::Off,
            sfw: ContentChip::Off,
            watched: ContentChip::Off,
            funscript: ContentChip::Off,
            subscriptions: ContentChip::Off,
            favorites: ContentChip::Off,
        }
    }
}

impl BrowseFilterState {
    pub fn from_route(sort: Option<&str>, tags: Option<&str>, creator: Option<&str>) -> Self {
        let mut s = Self::default();
        if let Some(sort) = sort {
            s.sort = VideoSort::from_api(sort);
        }
        if let Some(t) = tags {
            s.tags = t.to_string();
        }
        if let Some(c) = creator {
            s.creator = c.to_string();
        }
        s
    }

    pub fn to_feed(&self, page: u32) -> FeedParams {
        let mut p = FeedParams {
            page: Some(page),
            limit: Some(32),
            sort: Some(self.sort),
            tag_mode: Some(if self.tag_mode_and {
                "AND".into()
            } else {
                "OR".into()
            }),
            expand_tags: Some(self.expand_tags && !self.tag_mode_and),
            ..Default::default()
        };

        let tags = self.tags.trim();
        if !tags.is_empty() {
            if self.exclude_mode {
                p.exclude_tags = Some(tags.to_string());
            } else {
                p.tags = Some(tags.to_string());
            }
        }
        let excl = self.exclude_tags.trim();
        if !excl.is_empty() {
            p.exclude_tags = Some(excl.to_string());
        }
        let stars = self.stars.trim();
        if !stars.is_empty() {
            p.stars = Some(stars.to_string());
        }
        let music = self.music.trim();
        if !music.is_empty() {
            // Site uses musicArtist / musicTags; send both for coverage.
            p.music_artist = Some(music.to_string());
            p.music_tags = Some(music.to_string());
        }
        let creator = self.creator.trim();
        if !creator.is_empty() {
            p.creators = Some(creator.to_string());
            p.creator = Some(creator.to_string());
        }

        if let Some(from) = upload_from_preset(&self.uploaded) {
            p.upload_date_from = Some(from);
        }

        if let Some(m) = self.duration_min_min {
            p.duration_min = Some(m * 60);
        }
        if let Some(m) = self.duration_max_min {
            p.duration_max = Some(m * 60);
        }
        p.min_rating = self.rating_min;
        p.min_views = self.views_min;
        p.max_views = self.views_max;

        apply_quality(&mut p, &self.qualities);
        if !self.aspects.is_empty() {
            p.aspect = Some(self.aspects.join(","));
        }

        apply_chip(&mut p.include_gay, &mut p.exclude_gay, self.gay);
        apply_chip(&mut p.include_trans, &mut p.exclude_trans, self.trans);
        apply_chip(
            &mut p.include_voice_over,
            &mut p.exclude_voice_over,
            self.voice,
        );
        apply_chip(&mut p.include_remix, &mut p.exclude_remix, self.remix);
        apply_chip(
            &mut p.include_extreme,
            &mut p.exclude_extreme,
            self.extreme,
        );
        apply_chip(
            &mut p.include_promotional,
            &mut p.exclude_promotional,
            self.promo,
        );
        match self.sfw {
            ContentChip::Include => p.non_nude_only = true,
            ContentChip::Exclude => p.include_explicit = true,
            ContentChip::Off => {}
        }
        match self.watched {
            ContentChip::Include => p.watched_only = true,
            ContentChip::Exclude => p.exclude_watched = true,
            ContentChip::Off => {}
        }
        match self.funscript {
            ContentChip::Include => p.funscript_only = true,
            ContentChip::Exclude => p.exclude_funscript = true,
            ContentChip::Off => {}
        }
        if self.subscriptions == ContentChip::Include {
            p.subscribed_only = true;
        }
        if self.favorites == ContentChip::Include {
            p.favorites_only = true;
        }
        p
    }

    pub fn active_count(&self) -> u32 {
        let mut n = 0u32;
        if !self.tags.trim().is_empty() {
            n += 1;
        }
        if !self.exclude_tags.trim().is_empty() {
            n += 1;
        }
        if !self.stars.trim().is_empty() {
            n += 1;
        }
        if !self.music.trim().is_empty() {
            n += 1;
        }
        if !self.creator.trim().is_empty() {
            n += 1;
        }
        if !self.uploaded.is_empty() {
            n += 1;
        }
        if self.duration_min_min.is_some() || self.duration_max_min.is_some() {
            n += 1;
        }
        if self.rating_min.is_some() {
            n += 1;
        }
        if self.views_min.is_some() || self.views_max.is_some() {
            n += 1;
        }
        if !self.qualities.is_empty() {
            n += 1;
        }
        if !self.aspects.is_empty() {
            n += 1;
        }
        if self.tag_mode_and {
            n += 1;
        }
        if self.expand_tags {
            n += 1;
        }
        for c in [
            self.gay,
            self.trans,
            self.voice,
            self.remix,
            self.extreme,
            self.promo,
            self.sfw,
            self.watched,
            self.funscript,
            self.subscriptions,
            self.favorites,
        ] {
            if c != ContentChip::Off {
                n += 1;
            }
        }
        n
    }
}

fn apply_chip(include: &mut bool, exclude: &mut bool, chip: ContentChip) {
    match chip {
        ContentChip::Include => *include = true,
        ContentChip::Exclude => *exclude = true,
        ContentChip::Off => {}
    }
}

fn upload_from_preset(preset: &str) -> Option<String> {
    let now = chrono::Utc::now();
    let from = match preset {
        "today" => now.date_naive().and_hms_opt(0, 0, 0)?.and_utc(),
        "7days" => now - chrono::Duration::days(7),
        "30days" => now - chrono::Duration::days(30),
        "365days" => now - chrono::Duration::days(365),
        _ => return None,
    };
    Some(from.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

fn apply_quality(p: &mut FeedParams, qualities: &[&str]) {
    if qualities.is_empty() {
        return;
    }
    let mut min_h: Option<u32> = None;
    let mut max_h: Option<u32> = None;
    for q in qualities {
        let (lo, hi) = match *q {
            "4K" => (Some(2160), None),
            "QHD" => (Some(1440), Some(2159)),
            "FHD" => (Some(1080), Some(1439)),
            "HD" => (Some(720), Some(1079)),
            "SD" => (None, Some(719)),
            _ => continue,
        };
        if let Some(lo) = lo {
            min_h = Some(min_h.map(|m| m.min(lo)).unwrap_or(lo));
        }
        if let Some(hi) = hi {
            max_h = Some(max_h.map(|m| m.max(hi)).unwrap_or(hi));
        }
    }
    p.min_height = min_h;
    p.max_height = max_h;
}

fn chip_class(c: ContentChip) -> &'static str {
    match c {
        ContentChip::Off => "filter-chip",
        ContentChip::Include => "filter-chip on",
        ContentChip::Exclude => "filter-chip exclude",
    }
}

#[component]
pub fn BrowseFilterPanel(
    filters: Signal<BrowseFilterState>,
    on_apply: EventHandler<()>,
) -> Element {
    let f = filters();
    let active = f.active_count();

    rsx! {
        div { class: "filters-panel",
            div { class: "filters-toolbar",
                button {
                    class: "btn btn-ghost",
                    onclick: move |_| {
                        filters.with_mut(|s| s.open = !s.open);
                    },
                    if f.open { "Hide filters" } else { "Show filters" }
                    if active > 0 {
                        span { class: "filters-count", " {active}" }
                    }
                }
                div { class: "filters-sort-row",
                    for s in [
                        VideoSort::Newest,
                        VideoSort::TopRated,
                        VideoSort::MostLiked,
                        VideoSort::MostViews,
                        VideoSort::Oldest,
                    ] {
                        button {
                            class: if f.sort == s { "tab active" } else { "tab" },
                            onclick: move |_| {
                                filters.with_mut(|st| st.sort = s);
                                on_apply.call(());
                            },
                            "{s.label()}"
                        }
                    }
                }
            }

            if f.open {
                div { class: "filters-body",
                    div { class: "filters-grid",
                        // Basic
                        label { class: "filters-field",
                            span { "Uploaded" }
                            select {
                                value: "{f.uploaded}",
                                onchange: move |e| {
                                    filters.with_mut(|s| s.uploaded = e.value());
                                },
                                option { value: "", "Any time" }
                                option { value: "today", "Today" }
                                option { value: "7days", "Past week" }
                                option { value: "30days", "Past month" }
                                option { value: "365days", "Past year" }
                            }
                        }
                        label { class: "filters-field",
                            span { "Quality" }
                            div { class: "chip-row",
                                for q in ["SD", "HD", "FHD", "QHD", "4K"] {
                                    {
                                        let selected = f.qualities.contains(&q);
                                        rsx! {
                                            button {
                                                class: if selected { "filter-chip on" } else { "filter-chip" },
                                                onclick: move |_| {
                                                    filters.with_mut(|s| {
                                                        if let Some(i) = s.qualities.iter().position(|x| *x == q) {
                                                            s.qualities.remove(i);
                                                        } else {
                                                            s.qualities.push(q);
                                                        }
                                                    });
                                                },
                                                "{q}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        label { class: "filters-field",
                            span { "Aspect" }
                            div { class: "chip-row",
                                for a in ["16:9", "9:16", "4:3", "1:1"] {
                                    {
                                        let selected = f.aspects.iter().any(|x| *x == a);
                                        rsx! {
                                            button {
                                                class: if selected { "filter-chip on" } else { "filter-chip" },
                                                onclick: move |_| {
                                                    filters.with_mut(|s| {
                                                        if let Some(i) = s.aspects.iter().position(|x| *x == a) {
                                                            s.aspects.remove(i);
                                                        } else {
                                                            s.aspects.push(a);
                                                        }
                                                    });
                                                },
                                                "{a}"
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Search fields
                        label { class: "filters-field",
                            span { "Tags" }
                            input {
                                r#type: "text",
                                placeholder: "Add tag…",
                                value: "{f.tags}",
                                oninput: move |e| filters.with_mut(|s| s.tags = e.value()),
                            }
                            label { class: "filters-check",
                                input {
                                    r#type: "checkbox",
                                    checked: f.exclude_mode,
                                    onchange: move |_| filters.with_mut(|s| s.exclude_mode = !s.exclude_mode),
                                }
                                "Exclude from results"
                            }
                        }
                        label { class: "filters-field",
                            span { "Models" }
                            input {
                                r#type: "text",
                                placeholder: "Add model…",
                                value: "{f.stars}",
                                oninput: move |e| filters.with_mut(|s| s.stars = e.value()),
                            }
                        }
                        label { class: "filters-field",
                            span { "Music" }
                            input {
                                r#type: "text",
                                placeholder: "Search artist or song…",
                                value: "{f.music}",
                                oninput: move |e| filters.with_mut(|s| s.music = e.value()),
                            }
                        }
                        label { class: "filters-field",
                            span { "Creator" }
                            input {
                                r#type: "text",
                                placeholder: "Search creator…",
                                value: "{f.creator}",
                                oninput: move |e| filters.with_mut(|s| s.creator = e.value()),
                            }
                        }
                    }

                    // Duration presets
                    div { class: "filters-section",
                        span { class: "filters-label", "Duration" }
                        div { class: "chip-row",
                            button {
                                class: if f.duration_min_min.is_none() && f.duration_max_min.is_none() { "filter-chip on" } else { "filter-chip" },
                                onclick: move |_| filters.with_mut(|s| { s.duration_min_min = None; s.duration_max_min = None; }),
                                "Any"
                            }
                            button {
                                class: "filter-chip",
                                onclick: move |_| filters.with_mut(|s| { s.duration_min_min = None; s.duration_max_min = Some(4); }),
                                "<4 min"
                            }
                            button {
                                class: "filter-chip",
                                onclick: move |_| filters.with_mut(|s| { s.duration_min_min = Some(4); s.duration_max_min = Some(20); }),
                                "4–20 min"
                            }
                            button {
                                class: "filter-chip",
                                onclick: move |_| filters.with_mut(|s| { s.duration_min_min = Some(20); s.duration_max_min = Some(60); }),
                                "20–60 min"
                            }
                            button {
                                class: "filter-chip",
                                onclick: move |_| filters.with_mut(|s| { s.duration_min_min = Some(60); s.duration_max_min = None; }),
                                ">1 hour"
                            }
                        }
                    }

                    div { class: "filters-section",
                        span { class: "filters-label", "Rating" }
                        div { class: "chip-row",
                            button {
                                class: if f.rating_min.is_none() { "filter-chip on" } else { "filter-chip" },
                                onclick: move |_| filters.with_mut(|s| s.rating_min = None),
                                "Any"
                            }
                            for (label, min) in [("90%+", 90u32), ("80%+", 80), ("70%+", 70), ("50%+", 50)] {
                                button {
                                    class: if f.rating_min == Some(min) { "filter-chip on" } else { "filter-chip" },
                                    onclick: move |_| filters.with_mut(|s| s.rating_min = Some(min)),
                                    "{label}"
                                }
                            }
                        }
                    }

                    div { class: "filters-section",
                        span { class: "filters-label", "Views" }
                        div { class: "chip-row",
                            button {
                                class: if f.views_min.is_none() && f.views_max.is_none() { "filter-chip on" } else { "filter-chip" },
                                onclick: move |_| filters.with_mut(|s| { s.views_min = None; s.views_max = None; }),
                                "Any"
                            }
                            for (label, min) in [("1K+", 1_000u64), ("5K+", 5_000), ("10K+", 10_000), ("50K+", 50_000)] {
                                button {
                                    class: if f.views_min == Some(min) && f.views_max.is_none() { "filter-chip on" } else { "filter-chip" },
                                    onclick: move |_| filters.with_mut(|s| { s.views_min = Some(min); s.views_max = None; }),
                                    "{label}"
                                }
                            }
                            button {
                                class: "filter-chip",
                                onclick: move |_| filters.with_mut(|s| { s.views_min = None; s.views_max = Some(1_000); }),
                                "<1K"
                            }
                        }
                    }

                    div { class: "filters-section",
                        span { class: "filters-label", "Content" }
                        div { class: "chip-row wrap",
                            {
                                let chips: [(&str, ContentChip); 11] = [
                                    ("Gay", f.gay),
                                    ("Trans", f.trans),
                                    ("Voice", f.voice),
                                    ("Remix", f.remix),
                                    ("Extreme", f.extreme),
                                    ("Promo", f.promo),
                                    ("SFW", f.sfw),
                                    ("Watched", f.watched),
                                    ("Funscript", f.funscript),
                                    ("Subscriptions", f.subscriptions),
                                    ("Favorites", f.favorites),
                                ];
                                rsx! {
                                    for (label, chip) in chips {
                                        button {
                                            class: "{chip_class(chip)}",
                                            title: "Click to cycle include / exclude",
                                            onclick: move |_| {
                                                filters.with_mut(|s| {
                                                    match label {
                                                        "Gay" => s.gay = s.gay.cycle(),
                                                        "Trans" => s.trans = s.trans.cycle(),
                                                        "Voice" => s.voice = s.voice.cycle(),
                                                        "Remix" => s.remix = s.remix.cycle(),
                                                        "Extreme" => s.extreme = s.extreme.cycle(),
                                                        "Promo" => s.promo = s.promo.cycle(),
                                                        "SFW" => s.sfw = s.sfw.cycle(),
                                                        "Watched" => s.watched = s.watched.cycle(),
                                                        "Funscript" => s.funscript = s.funscript.cycle(),
                                                        "Subscriptions" => s.subscriptions = s.subscriptions.cycle(),
                                                        "Favorites" => s.favorites = s.favorites.cycle(),
                                                        _ => {}
                                                    }
                                                });
                                            },
                                            "{label}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "filters-footer",
                        label { class: "filters-check",
                            input {
                                r#type: "checkbox",
                                checked: f.expand_tags,
                                disabled: f.tag_mode_and,
                                onchange: move |_| filters.with_mut(|s| s.expand_tags = !s.expand_tags),
                            }
                            "Similar tags"
                        }
                        label { class: "filters-field inline",
                            span { "Match" }
                            select {
                                value: if f.tag_mode_and { "AND" } else { "OR" },
                                onchange: move |e| {
                                    filters.with_mut(|s| s.tag_mode_and = e.value() == "AND");
                                },
                                option { value: "OR", "Any (OR)" }
                                option { value: "AND", "All (AND)" }
                            }
                        }
                        button {
                            class: "btn btn-ghost",
                            onclick: move |_| {
                                let sort = filters().sort;
                                let open = filters().open;
                                *filters.write() = BrowseFilterState { sort, open, ..Default::default() };
                                on_apply.call(());
                            },
                            "Clear"
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| on_apply.call(()),
                            "Apply"
                        }
                    }
                }
            }
        }
    }
}
