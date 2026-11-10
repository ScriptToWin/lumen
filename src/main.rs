use eframe::egui::{
    self, Color32, ColorImage, FontId, RichText, Rounding, Sense, Stroke, TextureHandle,
    TextureOptions, Vec2,
};
use id3::TagLike;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

type PathList = Vec<PathBuf>;
type FavoriteSet = BTreeSet<PathBuf>;
type CustomPlaylistMap = BTreeMap<String, PathList>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Language {
    English,
    German,
    Japanese,
    Spanish,
    French,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RepeatMode {
    Off,
    All,
    One,
}

#[derive(Clone, PartialEq, Eq)]
enum ActiveTab {
    AllSongs,
    Favorites,
    FolderPlaylist(String),
    CustomPlaylist(String),
    Settings,
}

struct Track {
    title: String,
    artist: String,
    folder_playlist: String,
    extension: String,
    path: PathBuf,
    accent_color: Color32,
    cover: Option<TextureHandle>,
}

struct LumenApp {
    tracks: Vec<Track>,
    folder_playlists: Vec<String>,
    custom_playlists: CustomPlaylistMap,
    new_playlist_name: String,
    favorites: FavoriteSet,
    active_tab: ActiveTab,
    language: Language,
    repeat_mode: RepeatMode,
    search_query: String,
    current_track: Option<usize>,
    is_playing: bool,
    volume: f32,
    music_dir: PathBuf,
    status_message: String,
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Option<Sink>,
}

impl LumenApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::configure_fonts(&cc.egui_ctx);
        Self::configure_apple_theme(&cc.egui_ctx);

        let (_stream, stream_handle) =
        OutputStream::try_default().expect("Failed to initialize audio output");

        let music_dir = Self::resolve_music_dir();
        let (tracks, folder_playlists) = Self::scan_music_folder(&music_dir, &cc.egui_ctx);

        let mut custom_playlists = CustomPlaylistMap::new();
        custom_playlists.insert(String::from("Chill Mix"), PathList::new());

        Self {
            tracks,
            folder_playlists,
            custom_playlists,
            new_playlist_name: String::new(),
            favorites: FavoriteSet::new(),
            active_tab: ActiveTab::AllSongs,
            language: Language::English,
            repeat_mode: RepeatMode::Off,
            search_query: String::new(),
            current_track: None,
            is_playing: false,
            volume: 0.75,
            music_dir,
            status_message: String::new(),
            _stream,
            stream_handle,
            sink: None,
        }
    }

    fn tr<'a>(
        &self,
        en: &'a str,
        de: &'a str,
        ja: &'a str,
        es: &'a str,
        fr: &'a str,
    ) -> &'a str {
        match self.language {
            Language::English => en,
            Language::German => de,
            Language::Japanese => ja,
            Language::Spanish => es,
            Language::French => fr,
        }
    }

    fn perceived_volume(&self) -> f32 {
        (self.volume * self.volume).clamp(0.0, 1.0)
    }

    fn resolve_music_dir() -> PathBuf {
        if let Ok(home) = env::var("HOME") {
            let user_music = PathBuf::from(home).join("Music");
            if user_music.exists() {
                return user_music;
            }
        }
        let local_music = PathBuf::from("./music");
        if !local_music.exists() {
            let _ = fs::create_dir_all(&local_music);
        }
        local_music
    }

    fn find_font_file(root: &Path, patterns: &[&str]) -> Option<PathBuf> {
        let entries = fs::read_dir(root).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = Self::find_font_file(&path, patterns) {
                    return Some(found);
                }
            } else if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                let lower = name.to_lowercase();
                if (lower.ends_with(".ttf") || lower.ends_with(".otf") || lower.ends_with(".ttc"))
                    && patterns.iter().any(|p| lower.contains(p))
                    {
                        return Some(path);
                    }
            }
        }
        None
    }

    fn configure_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        let font_root = Path::new("/usr/share/fonts");

        if let Some(ui_path) = Self::find_font_file(
            font_root,
            &["inter-regular", "notosans-regular", "notosans[wght]", "cantarell", "liberationsans-regular"],
        ) {
            if let Ok(data) = fs::read(ui_path) {
                fonts
                .font_data
                .insert("system_ui".to_owned(), egui::FontData::from_owned(data));
                fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "system_ui".to_owned());
            }
        }

        if let Some(cjk_path) = Self::find_font_file(
            font_root,
            &["notosanscjk", "notosansjp", "vlgothic", "ipag", "droidsansfallback", "wqy"],
        ) {
            if let Ok(data) = fs::read(cjk_path) {
                fonts
                .font_data
                .insert("system_cjk".to_owned(), egui::FontData::from_owned(data));
                fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("system_cjk".to_owned());
            }
        }

        if let Some(sym_path) = Self::find_font_file(
            font_root,
            &["notosanssymbols2", "notosanssymbols", "dejavusans.ttf", "freesans.ttf"],
        ) {
            if let Ok(data) = fs::read(sym_path) {
                fonts
                .font_data
                .insert("system_symbols".to_owned(), egui::FontData::from_owned(data));
                fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("system_symbols".to_owned());
            }
        }

        ctx.set_fonts(fonts);
    }

    fn configure_apple_theme(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = Color32::from_rgb(245, 245, 247);
        visuals.window_fill = Color32::WHITE;
        visuals.widgets.noninteractive.bg_fill = Color32::WHITE;
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(235, 235, 240);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(225, 225, 232);
        visuals.widgets.active.bg_fill = Color32::from_rgb(250, 45, 72);
        visuals.selection.bg_fill = Color32::from_rgb(250, 45, 72);
        ctx.set_visuals(visuals);
    }

    fn scan_music_folder(dir: &Path, ctx: &egui::Context) -> (Vec<Track>, Vec<String>) {
        let mut tracks = Vec::new();
        let mut playlist_set = BTreeSet::new();
        let palette = [
            Color32::from_rgb(250, 45, 72),
            Color32::from_rgb(0, 122, 255),
            Color32::from_rgb(175, 82, 222),
            Color32::from_rgb(255, 149, 0),
            Color32::from_rgb(52, 199, 89),
            Color32::from_rgb(90, 200, 250),
        ];

        Self::collect_tracks(dir, dir, &mut tracks, &mut playlist_set, &palette, ctx);
        tracks.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        let playlists = playlist_set.into_iter().collect::<Vec<_>>();
        (tracks, playlists)
    }

    fn collect_tracks(
        root_dir: &Path,
        current_dir: &Path,
        tracks: &mut Vec<Track>,
        playlist_set: &mut BTreeSet<String>,
        palette: &[Color32],
        ctx: &egui::Context,
    ) {
        if let Ok(entries) = fs::read_dir(current_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::collect_tracks(root_dir, &path, tracks, playlist_set, palette, ctx);
                } else if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if ext_lower == "mp3" || ext_lower == "wav" {
                            let raw_stem = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown Track");

                            let clean_stem = raw_stem
                            .trim_end_matches("-cover")
                            .trim_start_matches(|c: char| c == '_' || c == '-' || c == ' ')
                            .to_string();

                            let fallback_title = if clean_stem.is_empty() {
                                raw_stem.to_string()
                            } else {
                                clean_stem
                            };

                            let mut title = fallback_title;
                            let mut artist = String::from("Unknown Artist");
                            let mut cover = None;

                            if ext_lower == "mp3" {
                                if let Ok(tag) = id3::Tag::read_from_path(&path) {
                                    if let Some(t) = tag.title() {
                                        if !t.trim().is_empty() {
                                            title = t.trim().to_string();
                                        }
                                    }
                                    if let Some(a) = tag.artist() {
                                        if !a.trim().is_empty() {
                                            artist = a.trim().to_string();
                                        }
                                    }
                                    if let Some(pic) = tag.pictures().next() {
                                        if let Ok(dyn_img) = image::load_from_memory(&pic.data) {
                                            let thumb = dyn_img.thumbnail(256, 256);
                                            let rgba = thumb.to_rgba8();
                                            let size =
                                            [rgba.width() as usize, rgba.height() as usize];
                                            let color_img = ColorImage::from_rgba_unmultiplied(
                                                size,
                                                rgba.as_flat_samples().as_slice(),
                                            );
                                            let tex_name = format!("cover_{}", tracks.len());
                                            cover = Some(ctx.load_texture(
                                                tex_name,
                                                color_img,
                                                TextureOptions::LINEAR,
                                            ));
                                        }
                                    }
                                }
                            }
                            let folder_playlist = if current_dir == root_dir {
                                String::from("General")
                            } else {
                                current_dir
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Folder")
                                .to_string()
                            };
                            playlist_set.insert(folder_playlist.clone());
                            let color_idx = tracks.len() % palette.len();
                            tracks.push(Track {
                                title,
                                artist,
                                folder_playlist,
                                extension: ext_lower.to_uppercase(),
                                        path,
                                        accent_color: palette[color_idx],
                                        cover,
                            });
                        }
                    }
                }
            }
        }
    }
    fn clear_ram_cache(&mut self, ctx: &egui::Context) {
        let mut freed_count = 0;
        for (idx, track) in self.tracks.iter_mut().enumerate() {
            if Some(idx) != self.current_track && track.cover.is_some() {
                track.cover = None;
                freed_count += 1;
            }
        }
        self.tracks.shrink_to_fit();
        self.folder_playlists.shrink_to_fit();
        for list in self.custom_playlists.values_mut() {
            list.shrink_to_fit();
        }
        ctx.memory_mut(|mem| {
            mem.caches = Default::default();
        });
        self.status_message = match self.language {
            Language::English => format!("Cleared {} cached covers & trimmed memory.", freed_count),
            Language::German => format!(
                "{} Cover-Texturen entfernt & RAM bereinigt.",
                freed_count
            ),
            Language::Japanese => format!("{}個のカバーキャッシュを削除しました。", freed_count),
            Language::Spanish => format!("Se liberaron {} portadas en caché.", freed_count),
            Language::French => format!("{} pochettes libérées du cache.", freed_count),
        };
    }
    fn play_track(&mut self, index: usize) {
        if index >= self.tracks.len() {
            return;
        }
        if let Some(old_sink) = self.sink.take() {
            old_sink.stop();
        }
        if let Ok(file) = File::open(&self.tracks[index].path) {
            if let Ok(source) = Decoder::new(BufReader::new(file)) {
                if let Ok(new_sink) = Sink::try_new(&self.stream_handle) {
                    new_sink.set_volume(self.perceived_volume());
                    new_sink.append(source);
                    new_sink.play();
                    self.sink = Some(new_sink);
                    self.current_track = Some(index);
                    self.is_playing = true;
                }
            }
        }
    }
    fn toggle_play_pause(&mut self) {
        if let Some(sink) = &self.sink {
            if self.is_playing {
                sink.pause();
                self.is_playing = false;
            } else {
                sink.play();
                self.is_playing = true;
            }
        } else if !self.tracks.is_empty() {
            self.play_track(0);
        }
    }
    fn handle_track_finished(&mut self) {
        if self.tracks.is_empty() {
            self.is_playing = false;
            return;
        }
        match self.repeat_mode {
            RepeatMode::One => {
                if let Some(idx) = self.current_track {
                    self.play_track(idx);
                }
            }
            RepeatMode::All => {
                self.next_track();
            }
            RepeatMode::Off => {
                if let Some(idx) = self.current_track {
                    if idx + 1 < self.tracks.len() {
                        self.play_track(idx + 1);
                    } else {
                        self.is_playing = false;
                    }
                }
            }
        }
    }
    fn next_track(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        let next = match self.current_track {
            Some(idx) => (idx + 1) % self.tracks.len(),
            None => 0,
        };
        self.play_track(next);
    }
    fn prev_track(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        let prev = match self.current_track {
            Some(0) | None => self.tracks.len() - 1,
            Some(idx) => idx - 1,
        };
        self.play_track(prev);
    }
    fn draw_artwork(
        ui: &mut egui::Ui,
        size: f32,
        color: Color32,
        cover: Option<&TextureHandle>,
        is_active: bool,
    ) {
        let (rect, _response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
        let painter = ui.painter();
        let rounding = Rounding::same(size * 0.16);
        if let Some(tex) = cover {
            painter.image(
                tex.id(),
                          rect,
                          egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                          Color32::WHITE,
            );
            painter.rect_stroke(
                rect,
                rounding,
                Stroke::new(1.0_f32, Color32::from_black_alpha(25)),
            );
        } else {
            painter.rect_filled(rect, rounding, color);
            painter.rect_stroke(
                rect,
                rounding,
                Stroke::new(1.0_f32, Color32::from_white_alpha(70)),
            );
            let icon_text = if is_active { "ılı" } else { "♪" };
            painter.text(
                rect.center(),
                         egui::Align2::CENTER_CENTER,
                         icon_text,
                         FontId::proportional(size * 0.44),
                         Color32::WHITE,
            );
        }
    }
}
impl eframe::App for LumenApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.is_playing {
            if let Some(sink) = &self.sink {
                if sink.empty() {
                    self.handle_track_finished();
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        // 1. LEFT SIDEBAR
        egui::SidePanel::left("sidebar_nav")
        .exact_width(210.0)
        .resizable(false)
        .frame(
            egui::Frame::none()
            .fill(Color32::from_rgb(238, 238, 242))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(222, 222, 228)))
            .inner_margin(egui::Margin::symmetric(14.0, 16.0)),
        )
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Lumen")
                .font(FontId::proportional(22.0))
                .strong()
                .color(Color32::from_rgb(29, 29, 31)),
            );
            ui.add_space(14.0);
            ui.label(
                RichText::new(self.tr(
                    "LIBRARY",
                    "MEDIATHEK",
                    "ライブラリ",
                    "BIBLIOTECA",
                    "BIBLIOTHÈQUE",
                ))
                .size(11.0)
                .strong()
                .color(Color32::from_rgb(134, 134, 139)),
            );
            ui.add_space(5.0);
            let nav_items = [
                (
                    ActiveTab::AllSongs,
                 self.tr(
                     "🎵  All Songs",
                     "🎵  Alle Titel",
                     "🎵  すべての曲",
                     "🎵  Canciones",
                     "🎵  Tous les titres",
                 ),
                ),
                (
                    ActiveTab::Favorites,
                 self.tr(
                     "♥  Favorites",
                     "♥  Favoriten",
                     "♥  お気に入り",
                     "♥  Favoritos",
                     "♥  Favoris",
                 ),
                ),
            ];
            for (tab, label) in nav_items {
                let selected = self.active_tab == tab;
                let fill = if selected {
                    Color32::from_rgb(250, 45, 72)
                } else {
                    Color32::TRANSPARENT
                };
                let text_color = if selected {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(29, 29, 31)
                };
                let btn = egui::Button::new(RichText::new(label).size(13.5).color(text_color))
                .fill(fill)
                .rounding(Rounding::same(8.0))
                .min_size(Vec2::new(ui.available_width(), 30.0));
                if ui.add(btn).clicked() {
                    self.active_tab = tab;
                }
                ui.add_space(3.0);
            }
            ui.add_space(12.0);
            ui.label(
                RichText::new(self.tr(
                    "MY PLAYLISTS",
                    "MEINE PLAYLISTEN",
                    "プレイリスト",
                    "MIS PLAYLISTS",
                    "MES PLAYLISTS",
                ))
                .size(11.0)
                .strong()
                .color(Color32::from_rgb(134, 134, 139)),
            );
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                let hint = self.tr(
                    "New playlist...",
                    "Neue Playlist...",
                    "新規プレイリスト...",
                    "Nueva playlist...",
                    "Nouvelle playlist...",
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_playlist_name)
                    .hint_text(hint)
                    .desired_width(136.0)
                    .margin(egui::Margin::symmetric(6.0, 4.0)),
                );
                if ui
                    .add(
                        egui::Button::new(RichText::new("+").strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(250, 45, 72))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(26.0, 24.0)),
                    )
                    .clicked()
                    {
                        let trimmed = self.new_playlist_name.trim().to_string();
                        if !trimmed.is_empty() {
                            self.custom_playlists.entry(trimmed).or_default();
                            self.new_playlist_name.clear();
                        }
                    }
            });
            ui.add_space(6.0);
            let custom_names = self.custom_playlists.keys().cloned().collect::<Vec<_>>();
            egui::ScrollArea::vertical()
            .id_salt("playlists_scroll")
            .max_height(220.0)
            .show(ui, |ui| {
                for pl in &custom_names {
                    let count = self.custom_playlists.get(pl).map(|v| v.len()).unwrap_or(0);
                    let tab = ActiveTab::CustomPlaylist(pl.clone());
                    let selected = self.active_tab == tab;
                    let fill = if selected {
                        Color32::from_rgb(250, 45, 72)
                    } else {
                        Color32::TRANSPARENT
                    };
                    let text_color = if selected {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(29, 29, 31)
                    };
                    let btn = egui::Button::new(
                        RichText::new(format!("🎶  {} ({})", pl, count))
                        .size(13.0)
                        .color(text_color),
                    )
                    .fill(fill)
                    .rounding(Rounding::same(8.0))
                    .min_size(Vec2::new(ui.available_width(), 28.0));
                    if ui.add(btn).clicked() {
                        self.active_tab = tab;
                    }
                    ui.add_space(3.0);
                }
                if !self.folder_playlists.is_empty() {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(self.tr(
                            "FOLDERS",
                            "ORDNER",
                            "フォルダ",
                            "CARPETAS",
                            "DOSSIERS",
                        ))
                        .size(10.5)
                        .strong()
                        .color(Color32::from_rgb(134, 134, 139)),
                    );
                    ui.add_space(4.0);
                    for pl in &self.folder_playlists {
                        let tab = ActiveTab::FolderPlaylist(pl.clone());
                        let selected = self.active_tab == tab;
                        let fill = if selected {
                            Color32::from_rgb(250, 45, 72)
                        } else {
                            Color32::TRANSPARENT
                        };
                        let text_color = if selected {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(29, 29, 31)
                        };
                        let btn = egui::Button::new(
                            RichText::new(format!("📁  {}", pl))
                            .size(12.5)
                            .color(text_color),
                        )
                        .fill(fill)
                        .rounding(Rounding::same(8.0))
                        .min_size(Vec2::new(ui.available_width(), 26.0));
                        if ui.add(btn).clicked() {
                            self.active_tab = tab;
                        }
                        ui.add_space(2.0);
                    }
                }
            });
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                let selected = self.active_tab == ActiveTab::Settings;
                let fill = if selected {
                    Color32::from_rgb(250, 45, 72)
                } else {
                    Color32::WHITE
                };
                let text_color = if selected {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(29, 29, 31)
                };
                let settings_btn = egui::Button::new(
                    RichText::new(self.tr(
                        "⚙  Settings",
                        "⚙  Einstellungen",
                        "⚙  設定",
                        "⚙  Ajustes",
                        "⚙  Paramètres",
                    ))
                    .size(13.5)
                    .color(text_color),
                )
                .fill(fill)
                .rounding(Rounding::same(8.0))
                .min_size(Vec2::new(ui.available_width(), 34.0));
                if ui.add(settings_btn).clicked() {
                    self.active_tab = ActiveTab::Settings;
                }
            });
        });
        // 2. RIGHT SIDEBAR
        egui::SidePanel::right("now_playing_panel")
        .exact_width(250.0)
        .resizable(false)
        .frame(
            egui::Frame::none()
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(225, 225, 230)))
            .inner_margin(egui::Margin::symmetric(20.0, 20.0)),
        )
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(self.tr(
                        "NOW PLAYING",
                        "AKTUELLER TITEL",
                        "再生中",
                        "REPRODUCIENDO",
                        "EN LECTURE",
                    ))
                    .size(11.0)
                    .strong()
                    .color(Color32::from_rgb(134, 134, 139)),
                );
                ui.add_space(16.0);
                if let Some(idx) = self.current_track {
                    if let Some(track) = self.tracks.get(idx) {
                        Self::draw_artwork(
                            ui,
                            180.0,
                            track.accent_color,
                            track.cover.as_ref(),
                                           self.is_playing,
                        );
                        ui.add_space(16.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(&track.title)
                                .size(16.0)
                                .strong()
                                .color(Color32::from_rgb(29, 29, 31)),
                            )
                            .truncate(),
                        );
                        ui.add_space(4.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(&track.artist)
                                .size(13.0)
                                .color(Color32::from_rgb(134, 134, 139)),
                            )
                            .truncate(),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(format!(
                                "{} • {}",
                                track.extension, track.folder_playlist
                            ))
                            .size(11.0)
                            .color(Color32::from_rgb(174, 174, 178)),
                        );
                    }
                } else {
                    Self::draw_artwork(
                        ui,
                        180.0,
                        Color32::from_rgb(230, 230, 235),
                                       None,
                                       false,
                    );
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(self.tr(
                            "Not Playing",
                            "Keine Wiedergabe",
                            "停止中",
                            "Sin reproducción",
                            "Aucune lecture",
                        ))
                        .size(15.5)
                        .strong()
                        .color(Color32::from_rgb(134, 134, 139)),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(self.tr(
                            "Select a song in the middle",
                            "Wähle einen Song in der Mitte",
                            "中央のリストから曲を選択",
                            "Elige una canción en el centro",
                            "Choisissez un titre au centre",
                        ))
                        .size(12.0)
                        .color(Color32::from_rgb(174, 174, 178)),
                    );
                }
                ui.add_space(22.0);
                // Transport Controls
                ui.allocate_ui_with_layout(
                    Vec2::new(160.0, 50.0),
                                           egui::Layout::left_to_right(egui::Align::Center),
                                           |ui| {
                                               if ui
                                                   .add(
                                                       egui::Button::new(RichText::new("⏮").size(20.0))
                                                       .frame(false)
                                                       .min_size(Vec2::splat(36.0)),
                                                   )
                                                   .clicked()
                                                   {
                                                       self.prev_track();
                                                   }
                                                   ui.add_space(12.0);
                                               let play_icon = if self.is_playing { "⏸" } else { "▶" };
                                               let play_btn = egui::Button::new(
                                                   RichText::new(play_icon).size(19.0).color(Color32::WHITE),
                                               )
                                               .fill(Color32::from_rgb(250, 45, 72))
                                               .rounding(Rounding::same(24.0))
                                               .min_size(Vec2::splat(48.0));
                                               if ui.add(play_btn).clicked() {
                                                   self.toggle_play_pause();
                                               }
                                               ui.add_space(12.0);
                                               if ui
                                                   .add(
                                                       egui::Button::new(RichText::new("⏭").size(20.0))
                                                       .frame(false)
                                                       .min_size(Vec2::splat(36.0)),
                                                   )
                                                   .clicked()
                                                   {
                                                       self.next_track();
                                                   }
                                           },
                );
                ui.add_space(14.0);
                // Repeat Button
                let (repeat_label, repeat_active) = match self.repeat_mode {
                    RepeatMode::Off => (
                        self.tr(
                            "🔁 Repeat: Off",
                            "🔁 Repeat: Aus",
                            "🔁 リピート: オフ",
                            "🔁 Repetir: No",
                            "🔁 Répéter: Non",
                        ),
                        false,
                    ),
                    RepeatMode::All => (
                        self.tr(
                            "🔁 Repeat: All",
                            "🔁 Repeat: Alle",
                            "🔁 リピート: 全曲",
                            "🔁 Repetir: Todo",
                            "🔁 Répéter: Tout",
                        ),
                        true,
                    ),
                    RepeatMode::One => (
                        self.tr(
                            "🔂 Repeat: 1x",
                            "🔂 Repeat: 1x",
                            "🔂 リピート: 1曲",
                            "🔂 Repetir: 1x",
                            "🔂 Répéter: 1x",
                        ),
                        true,
                    ),
                };
                let repeat_color = if repeat_active {
                    Color32::from_rgb(250, 45, 72)
                } else {
                    Color32::from_rgb(110, 110, 115)
                };
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(repeat_label).size(12.0).color(repeat_color),
                        )
                        .fill(Color32::from_rgb(245, 245, 247))
                        .rounding(Rounding::same(8.0))
                        .min_size(Vec2::new(145.0, 26.0)),
                    )
                    .clicked()
                    {
                        self.repeat_mode = match self.repeat_mode {
                            RepeatMode::Off => RepeatMode::All,
                            RepeatMode::All => RepeatMode::One,
                            RepeatMode::One => RepeatMode::Off,
                        };
                    }
                    ui.add_space(18.0);
                    // Volume Bar
                    ui.allocate_ui_with_layout(
                        Vec2::new(195.0, 24.0),
                                               egui::Layout::left_to_right(egui::Align::Center),
                                               |ui| {
                                                   ui.label(RichText::new("🔈").size(13.0));
                                                   let vol_slider = egui::Slider::new(&mut self.volume, 0.0..=1.0)
                                                   .show_value(false)
                                                   .trailing_fill(true);
                                                   if ui.add_sized(Vec2::new(145.0, 18.0), vol_slider).changed() {
                                                       if let Some(sink) = &self.sink {
                                                           sink.set_volume(self.perceived_volume());
                                                       }
                                                   }
                                                   ui.label(RichText::new("🔊").size(13.0));
                                               },
                    );
            });
        });
        // 3. CENTER PANEL
        egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
            .fill(Color32::from_rgb(245, 245, 247))
            .inner_margin(egui::Margin::symmetric(20.0, 18.0)),
        )
        .show(ctx, |ui| {
            if self.active_tab == ActiveTab::Settings {
                ui.label(
                    RichText::new(self.tr(
                        "Settings",
                        "Einstellungen",
                        "設定",
                        "Ajustes",
                        "Paramètres",
                    ))
                    .font(FontId::proportional(24.0))
                    .strong()
                    .color(Color32::from_rgb(29, 29, 31)),
                );
                ui.add_space(14.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Frame::none()
                    .fill(Color32::WHITE)
                    .rounding(Rounding::same(14.0))
                    .inner_margin(egui::Margin::symmetric(18.0, 16.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            RichText::new(self.tr(
                                "Language",
                                "Sprache",
                                "言語 (Language)",
                                                  "Idioma",
                                                  "Langue",
                            ))
                            .size(15.0)
                            .strong()
                            .color(Color32::from_rgb(29, 29, 31)),
                        );
                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .selectable_label(
                                    self.language == Language::English,
                                    "English",
                                )
                                .clicked()
                                {
                                    self.language = Language::English;
                                }
                                ui.add_space(8.0);
                            if ui
                                .selectable_label(
                                    self.language == Language::German,
                                    "Deutsch",
                                )
                                .clicked()
                                {
                                    self.language = Language::German;
                                }
                                ui.add_space(8.0);
                            if ui
                                .selectable_label(
                                    self.language == Language::Japanese,
                                    "日本語",
                                )
                                .clicked()
                                {
                                    self.language = Language::Japanese;
                                }
                                ui.add_space(8.0);
                            if ui
                                .selectable_label(
                                    self.language == Language::Spanish,
                                    "Español",
                                )
                                .clicked()
                                {
                                    self.language = Language::Spanish;
                                }
                                ui.add_space(8.0);
                            if ui
                                .selectable_label(
                                    self.language == Language::French,
                                    "Français",
                                )
                                .clicked()
                                {
                                    self.language = Language::French;
                                }
                        });
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(14.0);
                        ui.label(
                            RichText::new(self.tr(
                                "Memory & Cache",
                                "Arbeitsspeicher & Cache",
                                "メモリとキャッシュ",
                                "Memoria y Caché",
                                "Mémoire et Cache",
                            ))
                            .size(15.0)
                            .strong()
                            .color(Color32::from_rgb(29, 29, 31)),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(self.tr(
                                "Frees cached album artwork textures and trims internal buffers to save RAM.",
                                "Entfernt geladene Album-Cover aus dem Grafikspeicher und verkleinert interne Puffer.",
                                "キャッシュされたカバー画像を解放してRAMを節約します。",
                                "Libera las portadas en caché y reduce los búferes para ahorrar RAM.",
                                "Libère les pochettes en cache et réduit la mémoire utilisée.",
                            ))
                            .size(12.5)
                            .color(Color32::from_rgb(134, 134, 139)),
                        );
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let clear_btn = egui::Button::new(
                                RichText::new(self.tr(
                                    "🧹 Clear RAM",
                                    "🧹 RAM bereinigen",
                                    "🧹 RAMをクリア",
                                    "🧹 Limpiar RAM",
                                    "🧹 Vider la RAM",
                                ))
                                .size(13.0)
                                .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(250, 45, 72))
                            .rounding(Rounding::same(8.0));
                            if ui.add(clear_btn).clicked() {
                                self.clear_ram_cache(ctx);
                            }
                            if ui
                                .button(self.tr(
                                    "↻ Reload Covers",
                                    "↻ Cover neu laden",
                                    "↻ カバー再読込",
                                    "↻ Recargar portadas",
                                    "↻ Recharger",
                                ))
                                .clicked()
                                {
                                    let (t, p) = Self::scan_music_folder(&self.music_dir, ctx);
                                    self.tracks = t;
                                    self.folder_playlists = p;
                                    self.status_message.clear();
                                }
                        });
                        if !self.status_message.is_empty() {
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(&self.status_message)
                                .size(12.0)
                                .color(Color32::from_rgb(52, 199, 89)),
                            );
                        }
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(14.0);
                        ui.label(
                            RichText::new(self.tr(
                                "Music Folder",
                                "Musikordner",
                                "音楽フォルダ",
                                "Carpeta de música",
                                "Dossier musique",
                            ))
                            .size(15.0)
                            .strong()
                            .color(Color32::from_rgb(29, 29, 31)),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(format!("{}", self.music_dir.display()))
                            .size(13.0)
                            .color(Color32::from_rgb(134, 134, 139)),
                        );
                    });
                    ui.add_space(14.0);
                    egui::Frame::none()
                    .fill(Color32::WHITE)
                    .rounding(Rounding::same(14.0))
                    .inner_margin(egui::Margin::symmetric(18.0, 16.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            RichText::new(self.tr(
                                "About Lumen",
                                "Über Lumen",
                                "Lumen について",
                                "Acerca de Lumen",
                                "À propos de Lumen",
                            ))
                            .size(15.0)
                            .strong()
                            .color(Color32::from_rgb(29, 29, 31)),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("All rights belong to Ceyro • © 2026 Ceyro")
                            .size(13.0)
                            .color(Color32::from_rgb(134, 134, 139)),
                        );
                        ui.add_space(10.0);
                        ui.hyperlink_to(
                            RichText::new("▶ YouTube: Ceyro")
                            .size(13.5)
                            .strong()
                            .color(Color32::from_rgb(250, 45, 72)),
                                        "youtube.com",
                        );
                    });
                });
                return;
            }
            // Header für Songliste
            let heading = match &self.active_tab {
                ActiveTab::AllSongs => self
                .tr(
                    "All Songs",
                    "Alle Titel",
                    "すべての曲",
                    "Canciones",
                    "Tous les titres",
                )
                .to_string(),
              ActiveTab::Favorites => self
              .tr(
                  "Favorites",
                  "Favoriten",
                  "お気に入り",
                  "Favoritos",
                  "Favoris",
              )
              .to_string(),
              ActiveTab::FolderPlaylist(name) => name.clone(),
              ActiveTab::CustomPlaylist(name) => name.clone(),
              ActiveTab::Settings => String::new(),
            };
            let search_hint = self
            .tr(
                "Search...",
                "Suchen...",
                "検索...",
                "Buscar...",
                "Rechercher...",
            )
            .to_string();
            let mut delete_custom_pl = None;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(heading)
                    .font(FontId::proportional(22.0))
                    .strong()
                    .color(Color32::from_rgb(29, 29, 31)),
                );
                if let ActiveTab::CustomPlaylist(pl_name) = &self.active_tab {
                    ui.add_space(8.0);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(self.tr(
                                    "Delete Playlist",
                                    "Playlist löschen",
                                    "削除",
                                    "Eliminar",
                                    "Supprimer",
                                ))
                                .size(11.5)
                                .color(Color32::from_rgb(250, 45, 72)),
                            )
                            .fill(Color32::WHITE)
                            .rounding(Rounding::same(6.0)),
                        )
                        .clicked()
                        {
                            delete_custom_pl = Some(pl_name.clone());
                        }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let refresh_btn = egui::Button::new(
                        RichText::new("↻")
                        .size(14.0)
                        .color(Color32::from_rgb(250, 45, 72)),
                    )
                    .fill(Color32::WHITE)
                    .rounding(Rounding::same(8.0))
                    .min_size(Vec2::splat(28.0));
                    if ui.add(refresh_btn).clicked() {
                        let (t, p) = Self::scan_music_folder(&self.music_dir, ctx);
                        self.tracks = t;
                        self.folder_playlists = p;
                    }
                    ui.add_space(6.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text(search_hint)
                        .desired_width(130.0)
                        .margin(egui::Margin::symmetric(8.0, 5.0)),
                    );
                });
            });
            if let Some(pl_del) = delete_custom_pl {
                self.custom_playlists.remove(&pl_del);
                self.active_tab = ActiveTab::AllSongs;
                return;
            }
            ui.add_space(12.0);
            let query_lower = self.search_query.to_lowercase();
            let mut display_entries = Vec::new();
            if let ActiveTab::CustomPlaylist(pl_name) = &self.active_tab {
                if let Some(paths) = self.custom_playlists.get(pl_name) {
                    for (entry_pos, path) in paths.iter().enumerate() {
                        if let Some(track_idx) =
                            self.tracks.iter().position(|t| &t.path == path)
                            {
                                let t = &self.tracks[track_idx];
                                if query_lower.is_empty()
                                    || t.title.to_lowercase().contains(&query_lower)
                                    || t.artist.to_lowercase().contains(&query_lower)
                                    {
                                        display_entries.push((track_idx, Some(entry_pos)));
                                    }
                            }
                    }
                }
            } else {
                for (idx, t) in self.tracks.iter().enumerate() {
                    let tab_match = match &self.active_tab {
                        ActiveTab::AllSongs => true,
              ActiveTab::Favorites => self.favorites.contains(&t.path),
              ActiveTab::FolderPlaylist(pl) => &t.folder_playlist == pl,
              _ => false,
                    };
                    let search_match = query_lower.is_empty()
                    || t.title.to_lowercase().contains(&query_lower)
                    || t.artist.to_lowercase().contains(&query_lower);
                    if tab_match && search_match {
                        display_entries.push((idx, None));
                    }
                }
            }
            if display_entries.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new(self.tr(
                            "No tracks here yet. Use '+' on any song to add it to a playlist.",
                            "Noch keine Songs hier. Nutze '+' an einem Song, um ihn einer Playlist hinzuzufügen.",
                            "曲がありません。「+」からプレイリストに追加できます。",
                            "No hay canciones aquí. Usa '+' para añadir una.",
                            "Aucun titre ici. Utilisez '+' pour en ajouter.",
                        ))
                        .size(13.5)
                        .color(Color32::from_rgb(134, 134, 139)),
                    );
                });
                return;
            }
            let custom_playlist_names =
            self.custom_playlists.keys().cloned().collect::<Vec<_>>();
            egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut clicked_index = None;
                let mut toggle_fav_path = None;
                let mut add_to_playlist_action = None;
                let mut remove_from_playlist_pos = None;
                for (row_num, (idx, custom_pos)) in display_entries.iter().enumerate() {
                    let track = &self.tracks[*idx];
                    let is_current = self.current_track == Some(*idx);
                    let is_fav = self.favorites.contains(&track.path);
                    let row_fill = if is_current {
                        Color32::WHITE
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 185)
                    };
                    let stroke = if is_current {
                        Stroke::new(
                            1.0_f32,
                            Color32::from_rgba_unmultiplied(250, 45, 72, 110),
                        )
                    } else {
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 10))
                    };
                    let frame = egui::Frame::none()
                    .fill(row_fill)
                    .stroke(stroke)
                    .rounding(Rounding::same(12.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0));
                    frame.show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let left_width = (ui.available_width() - 86.0).max(80.0);
                            let left_resp = ui.allocate_ui_with_layout(
                                Vec2::new(left_width, 42.0),
                                                                       egui::Layout::left_to_right(egui::Align::Center),
                                                                       |ui| {
                                                                           Self::draw_artwork(
                                                                               ui,
                                                                               42.0,
                                                                               track.accent_color,
                                                                               track.cover.as_ref(),
                                                                                              is_current && self.is_playing,
                                                                           );
                                                                           ui.add_space(12.0);
                                                                           ui.vertical(|ui| {
                                                                               let title_color = if is_current {
                                                                                   Color32::from_rgb(250, 45, 72)
                                                                               } else {
                                                                                   Color32::from_rgb(29, 29, 31)
                                                                               };
                                                                               ui.add(
                                                                                   egui::Label::new(
                                                                                       RichText::new(&track.title)
                                                                                       .size(14.0)
                                                                                       .strong()
                                                                                       .color(title_color),
                                                                                   )
                                                                                   .truncate(),
                                                                               );
                                                                               ui.add(
                                                                                   egui::Label::new(
                                                                                       RichText::new(format!(
                                                                                           "{} • {}",
                                                                                           track.artist, track.extension
                                                                                       ))
                                                                                       .size(11.5)
                                                                                       .color(Color32::from_rgb(134, 134, 139)),
                                                                                   )
                                                                                   .truncate(),
                                                                               );
                                                                           });
                                                                       },
                            );
                            let play_click = ui.interact(
                                left_resp.response.rect,
                                ui.id().with(("play_area", row_num, *idx)),
                                                         Sense::click(),
                            );
                            if play_click.hovered() {
                                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if play_click.clicked() {
                                clicked_index = Some(*idx);
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                           |ui| {
                                               let heart = if is_fav { "♥" } else { "♡" };
                                               let heart_color = if is_fav {
                                                   Color32::from_rgb(250, 45, 72)
                                               } else {
                                                   Color32::from_rgb(150, 150, 156)
                                               };
                                               if ui
                                                   .add(
                                                       egui::Button::new(
                                                           RichText::new(heart)
                                                           .size(17.0)
                                                           .color(heart_color),
                                                       )
                                                       .frame(false)
                                                       .min_size(Vec2::splat(28.0)),
                                                   )
                                                   .clicked()
                                                   {
                                                       toggle_fav_path = Some(track.path.clone());
                                                   }
                                                   ui.add_space(4.0);
                                                   ui.menu_button(
                                                       RichText::new("+")
                                                       .size(17.0)
                                                       .strong()
                                                       .color(Color32::from_rgb(90, 90, 95)),
                                                                  |ui| {
                                                                      ui.label(
                                                                          RichText::new(self.tr(
                                                                              "Add to Playlist:",
                                                                              "Zur Playlist hinzufügen:",
                                                                              "プレイリストに追加:",
                                                                              "Añadir a playlist:",
                                                                              "Ajouter à la playlist:",
                                                                          ))
                                                                          .size(11.5)
                                                                          .strong(),
                                                                      );
                                                                      ui.separator();
                                                                      for pl_name in &custom_playlist_names {
                                                                          if ui
                                                                              .button(format!("🎶 {}", pl_name))
                                                                              .clicked()
                                                                              {
                                                                                  add_to_playlist_action = Some((
                                                                                      pl_name.clone(),
                                                                                                                 track.path.clone(),
                                                                                  ));
                                                                                  ui.close_menu();
                                                                              }
                                                                      }
                                                                  },
                                                   );
                                                   if let Some(pos) = custom_pos {
                                                       if ui
                                                           .add(
                                                               egui::Button::new(
                                                                   RichText::new("✕")
                                                                   .size(13.0)
                                                                   .color(Color32::from_rgb(
                                                                       150, 150, 156,
                                                                   )),
                                                               )
                                                               .frame(false)
                                                               .min_size(Vec2::splat(24.0)),
                                                           )
                                                           .clicked()
                                                           {
                                                               remove_from_playlist_pos = Some(*pos);
                                                           }
                                                   }
                                           },
                            );
                        });
                    });
                    ui.add_space(6.0);
                }
                if let Some((pl_name, path)) = add_to_playlist_action {
                    if let Some(list) = self.custom_playlists.get_mut(&pl_name) {
                        list.push(path);
                    }
                } else if let Some(pos) = remove_from_playlist_pos {
                    if let ActiveTab::CustomPlaylist(pl_name) = &self.active_tab {
                        if let Some(list) = self.custom_playlists.get_mut(pl_name) {
                            if pos < list.len() {
                                list.remove(pos);
                            }
                        }
                    }
                } else if let Some(p) = toggle_fav_path {
                    if !self.favorites.remove(&p) {
                        self.favorites.insert(p);
                    }
                } else if let Some(idx) = clicked_index {
                    self.play_track(idx);
                }
            });
        });
    }
}
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
        .with_inner_size(Vec2::new(980.0, 640.0))
        .with_min_inner_size(Vec2::new(840.0, 520.0))
        .with_title("Lumen"),
        ..Default::default()
    };
    eframe::run_native(
        "Lumen",
        options,
        Box::new(|cc| Ok(Box::new(LumenApp::new(cc)))),
    )
}
