//! Backgrounds: still pictures (GSettings `org.gnome.desktop.background`) and animated ones, which
//! the session service `com.nodalix.Settings` sets up and plays through the animated background
//! player. Thumbnails are decoded off the main loop and small, so a folder of 4K pictures does
//! not freeze the window or fill the memory.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gdk, gio, glib, prelude::*},
};

use ajustes_core::wallpapers::{self, PlayerState, Wallpaper, PAUSE_CHOICES};

use super::column;
use super::gs::{self, s};
use crate::i18n::tr;

const BUS: &str = "com.nodalix.Settings";
const PATH: &str = "/com/nodalix/Settings";
const INTERFACE: &str = "com.nodalix.Settings1";
const BACKGROUND: &str = "org.gnome.desktop.background";

const TILE_W: i32 = 144;
const TILE_H: i32 = 108;
const MAX_PICTURES: usize = 60;
const EXTENSIONS: [&str; 6] = ["jpg", "jpeg", "png", "webp", "jxl", "svg"];

const FIT: [(&str, gs::Val); 5] = [
    s("Fill the screen", "zoom"),
    s("Fit inside", "scaled"),
    s("Stretch", "stretched"),
    s("Center", "centered"),
    s("Tile", "wallpaper"),
];

/// Pictures in the system and personal background folders, one level down, in name order.
pub fn pictures(roots: &[PathBuf]) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                if depth > 0 {
                    walk(&path, depth - 1, out);
                }
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
            {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    for root in roots {
        walk(root, 1, &mut out);
    }
    out.truncate(MAX_PICTURES);
    out
}

fn roots() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("/usr/share/backgrounds")];
    roots.push(glib::user_data_dir().join("backgrounds"));
    roots
}

/// A picture scaled to the tile, as raw pixels.
struct Pixels {
    width: i32,
    height: i32,
    stride: usize,
    alpha: bool,
    data: Vec<u8>,
}

/// Where the picture of a video this app made itself is kept.
fn thumbnail_file(video: &Path) -> Option<PathBuf> {
    use std::hash::{Hash, Hasher};
    let meta = std::fs::metadata(video).ok()?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    video.hash(&mut hasher);
    meta.len().hash(&mut hasher);
    meta.modified().ok()?.hash(&mut hasher);
    Some(
        glib::user_cache_dir()
            .join("nodalix")
            .join("settings-thumbnails")
            .join(format!("{:016x}.jpg", hasher.finish())),
    )
}

/// The first frame of a video as a small picture, made with `ffmpeg` when the session service
/// has not (it may be an older one, or the video may have failed there). `None` if there is no
/// `ffmpeg` or it cannot read the video.
async fn make_thumbnail(video: PathBuf) -> Option<PathBuf> {
    let target = thumbnail_file(&video)?;
    if target.is_file() {
        return Some(target);
    }
    std::fs::create_dir_all(target.parent()?).ok()?;
    let temporary = target.with_extension("part.jpg");
    let (input, output) = (
        video.to_string_lossy().into_owned(),
        temporary.to_string_lossy().into_owned(),
    );
    for position in ["1", "0"] {
        let result = super::cmd::capture(
            &[
                "ffmpeg",
                "-nostdin",
                "-hide_banner",
                "-loglevel",
                "error",
                "-ss",
                position,
                "-i",
                &input,
                "-map",
                "0:V:0",
                "-frames:v",
                "1",
                "-vf",
                "scale=288:216:force_original_aspect_ratio=increase,crop=288:216",
                "-q:v",
                "3",
                "-y",
                &output,
            ],
            None,
        )
        .await;
        if result.ok && std::fs::metadata(&temporary).is_ok_and(|m| m.len() > 0) {
            std::fs::rename(&temporary, &target).ok()?;
            return Some(target);
        }
    }
    let _ = std::fs::remove_file(&temporary);
    None
}

fn decode(path: &Path) -> Option<Pixels> {
    use gtk::gdk_pixbuf::Pixbuf;
    // Scaled to cover the tile, then cropped from the middle: a tile's size must be the
    // picture's own size, or GTK makes the tile as large as the picture.
    let (_, w, h) = Pixbuf::file_info(path)?;
    if w <= 0 || h <= 0 {
        return None;
    }
    let scale = f64::max(
        f64::from(TILE_W) / f64::from(w),
        f64::from(TILE_H) / f64::from(h),
    );
    let (cover_w, cover_h) = (
        ((f64::from(w) * scale).ceil() as i32).max(TILE_W),
        ((f64::from(h) * scale).ceil() as i32).max(TILE_H),
    );
    let covered = Pixbuf::from_file_at_scale(path, cover_w, cover_h, false).ok()?;
    let x = ((covered.width() - TILE_W) / 2).max(0);
    let y = ((covered.height() - TILE_H) / 2).max(0);
    let cropped = covered
        .new_subpixbuf(
            x,
            y,
            TILE_W.min(covered.width()),
            TILE_H.min(covered.height()),
        )
        .copy()?;
    Some(Pixels {
        width: cropped.width(),
        height: cropped.height(),
        stride: cropped.rowstride() as usize,
        alpha: cropped.has_alpha(),
        data: cropped.read_pixel_bytes().to_vec(),
    })
}

fn texture(pixels: Pixels) -> gdk::Texture {
    let format = if pixels.alpha {
        gdk::MemoryFormat::R8g8b8a8
    } else {
        gdk::MemoryFormat::R8g8b8
    };
    gdk::MemoryTexture::new(
        pixels.width,
        pixels.height,
        format,
        &glib::Bytes::from_owned(pixels.data),
        pixels.stride,
    )
    .upcast()
}

/// A gallery of picture tiles. Each tile is a button; the one that is in use shows a check.
struct Gallery {
    flow: gtk::FlowBox,
    tiles: RefCell<Vec<(String, gtk::Image)>>,
}

impl Gallery {
    fn new() -> Rc<Self> {
        let flow = gtk::FlowBox::new();
        flow.set_selection_mode(gtk::SelectionMode::None);
        flow.set_homogeneous(true);
        flow.set_min_children_per_line(1);
        flow.set_max_children_per_line(8);
        flow.set_row_spacing(12);
        flow.set_column_spacing(12);
        flow.set_halign(gtk::Align::Center);
        Rc::new(Self {
            flow,
            tiles: RefCell::default(),
        })
    }

    fn clear(&self) {
        self.flow.remove_all();
        self.tiles.borrow_mut().clear();
    }

    /// Adds a tile identified by `key`; its picture is decoded in the background.
    fn add(
        &self,
        key: &str,
        picture: PathBuf,
        video: Option<PathBuf>,
        tooltip: &str,
        on_pick: impl Fn() + 'static,
    ) {
        let image = gtk::Picture::new();
        image.set_content_fit(gtk::ContentFit::Cover);
        image.set_can_shrink(true);
        image.set_size_request(TILE_W, TILE_H);
        image.set_halign(gtk::Align::Center);
        image.set_valign(gtk::Align::Center);
        image.set_overflow(gtk::Overflow::Hidden);
        image.add_css_class("settings-tile-picture");
        let check = gtk::Image::from_icon_name("object-select-symbolic");
        check.set_halign(gtk::Align::End);
        check.set_valign(gtk::Align::End);
        check.set_margin_end(6);
        check.set_margin_bottom(6);
        check.add_css_class("settings-tile-check");
        check.set_visible(false);
        // A picture that is not there yet (a video whose first frame is still being made) shows
        // a video icon instead of an empty box.
        let placeholder = gtk::Image::from_icon_name("video-x-generic-symbolic");
        placeholder.set_pixel_size(32);
        placeholder.set_halign(gtk::Align::Center);
        placeholder.set_valign(gtk::Align::Center);
        placeholder.set_can_target(false);
        placeholder.set_opacity(0.6);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&image));
        overlay.add_overlay(&placeholder);
        overlay.add_overlay(&check);
        let button = gtk::Button::new();
        button.set_child(Some(&overlay));
        button.add_css_class("flat");
        button.add_css_class("settings-tile");
        button.set_tooltip_text(Some(tooltip));
        button.connect_clicked(move |_| on_pick());
        self.flow.append(&button);
        self.tiles.borrow_mut().push((key.to_owned(), check));
        glib::spawn_future_local(async move {
            let mut pixels = gio::spawn_blocking(move || decode(&picture))
                .await
                .ok()
                .flatten();
            if pixels.is_none() {
                // No picture from the service: make one from the video itself.
                if let Some(thumbnail) = match video {
                    Some(video) => make_thumbnail(video).await,
                    None => None,
                } {
                    pixels = gio::spawn_blocking(move || decode(&thumbnail))
                        .await
                        .ok()
                        .flatten();
                }
            }
            if let Some(pixels) = pixels {
                image.set_paintable(Some(&texture(pixels)));
                placeholder.set_visible(false);
            }
        });
    }

    fn mark(&self, active: &str) {
        for (key, check) in self.tiles.borrow().iter() {
            check.set_visible(!active.is_empty() && key == active);
        }
    }
}

fn uri_of(path: &Path) -> String {
    gio::File::for_path(path).uri().to_string()
}

/// The service's reply as text, or `None` (service not running, call refused).
async fn ask(proxy: &gio::DBusProxy, method: &str, args: Option<&glib::Variant>) -> Option<String> {
    let reply = proxy
        .call_future(method, args, gio::DBusCallFlags::NONE, 4_000)
        .await
        .ok()?;
    reply.child_value(0).str().map(str::to_owned)
}

/// The background sections of the Appearance page: pictures, the lock screen and animated ones.
pub fn section(toasts: &ndl::ToastOverlay) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    let settings = gio::Settings::new(BACKGROUND);
    let proxy: Rc<RefCell<Option<gio::DBusProxy>>> = Rc::default();

    // Still pictures.
    let stills = Gallery::new();
    let still_group = ndl::PreferencesGroup::builder()
        .title(tr("Desktop background"))
        .build();
    let current_uri = {
        let settings = settings.clone();
        move || settings.string("picture-uri").to_string()
    };
    let choose = ndl::ActionRow::builder()
        .title(tr("Choose a picture…"))
        .build();
    choose.set_activatable(true);
    choose.add_prefix(&gtk::Image::from_icon_name("folder-pictures-symbolic"));
    still_group.add(&choose);
    if let Some(g) = gs::open(BACKGROUND) {
        if let Some(row) = g.choice("picture-options", tr("Fit"), &FIT) {
            still_group.add(&row);
        }
    }
    content.append(&still_group);
    let gallery_box = column();
    gallery_box.set_margin_top(0);
    gallery_box.set_margin_bottom(0);
    gallery_box.set_margin_start(0);
    gallery_box.set_margin_end(0);
    gallery_box.append(&stills.flow);
    content.append(&gallery_box);

    let set_still = {
        let (settings, proxy) = (settings.clone(), proxy.clone());
        Rc::new(move |path: &Path| {
            let uri = uri_of(path);
            let _ = settings.set_string("picture-uri", &uri);
            let _ = settings.set_string("picture-uri-dark", &uri);
            // A still picture replaces an animated one.
            if let Some(proxy) = proxy.borrow().clone() {
                glib::spawn_future_local(async move {
                    let _ = proxy
                        .call_future("StopWallpaper", None, gio::DBusCallFlags::NONE, 4_000)
                        .await;
                });
            }
        })
    };
    for path in pictures(&roots()) {
        let name = path
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (set, key) = (set_still.clone(), uri_of(&path));
        let picked = path.clone();
        let stills2 = stills.clone();
        let key2 = key.clone();
        stills.add(&key, path, None, &name, move || {
            set(&picked);
            stills2.mark(&key2);
        });
    }
    stills.mark(&current_uri());
    {
        let stills = stills.clone();
        let settings2 = settings.clone();
        settings.connect_changed(Some("picture-uri"), move |_, _| {
            stills.mark(&settings2.string("picture-uri"));
        });
    }
    {
        let (set_still, toasts) = (set_still.clone(), toasts.clone());
        choose.connect_activated(move |row| {
            let dialog = gtk::FileDialog::builder()
                .title(tr("Choose a picture…"))
                .build();
            let filter = gtk::FileFilter::new();
            filter.add_mime_type("image/*");
            filter.set_name(Some(tr("Pictures")));
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));
            let window = row.root().and_downcast::<gtk::Window>();
            let (set_still, toasts) = (set_still.clone(), toasts.clone());
            glib::spawn_future_local(async move {
                match dialog.open_future(window.as_ref()).await {
                    Ok(file) => {
                        if let Some(path) = file.path() {
                            set_still(&path);
                        }
                    }
                    Err(error) => {
                        if !error.matches(gtk::DialogError::Dismissed) {
                            toasts.add_toast(ndl::Toast::new(error.message()));
                        }
                    }
                }
            });
        });
    }

    // The lock screen picture.
    if let Some(lock) = gs::open("org.gnome.desktop.screensaver") {
        if lock.has("picture-uri") {
            let screensaver = lock.settings().clone();
            let group = ndl::PreferencesGroup::builder()
                .title(tr("Lock screen"))
                .build();
            let row = ndl::ActionRow::builder()
                .title(tr("Lock screen picture"))
                .build();
            row.add_prefix(&gtk::Image::from_icon_name("system-lock-screen-symbolic"));
            let name_of = |uri: &str| {
                if uri.is_empty() {
                    tr("Same as desktop").to_owned()
                } else {
                    gio::File::for_uri(uri)
                        .basename()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                }
            };
            row.set_subtitle(&name_of(&screensaver.string("picture-uri")));
            let choose_lock = gtk::Button::with_label(tr("Choose…"));
            choose_lock.set_valign(gtk::Align::Center);
            let same = gtk::Button::with_label(tr("Same as desktop"));
            same.set_valign(gtk::Align::Center);
            row.add_suffix(&same);
            row.add_suffix(&choose_lock);
            group.add(&row);
            content.append(&group);
            {
                let (row, screensaver) = (row.clone(), screensaver.clone());
                screensaver
                    .clone()
                    .connect_changed(Some("picture-uri"), move |_, _| {
                        row.set_subtitle(&name_of(&screensaver.string("picture-uri")));
                    });
            }
            {
                let screensaver = screensaver.clone();
                same.connect_clicked(move |_| {
                    let _ = screensaver.set_string("picture-uri", "");
                });
            }
            let toasts = toasts.clone();
            choose_lock.connect_clicked(move |button| {
                let dialog = gtk::FileDialog::builder()
                    .title(tr("Lock screen picture"))
                    .build();
                let filter = gtk::FileFilter::new();
                filter.add_mime_type("image/*");
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                dialog.set_filters(Some(&filters));
                let window = button.root().and_downcast::<gtk::Window>();
                let (screensaver, toasts) = (screensaver.clone(), toasts.clone());
                glib::spawn_future_local(async move {
                    match dialog.open_future(window.as_ref()).await {
                        Ok(file) => {
                            let _ = screensaver.set_string("picture-uri", file.uri().as_str());
                        }
                        Err(error) => {
                            if !error.matches(gtk::DialogError::Dismissed) {
                                toasts.add_toast(ndl::Toast::new(error.message()));
                            }
                        }
                    }
                });
            });
        }
    }

    // Animated backgrounds: only when the service says the player is installed.
    let animated_group = ndl::PreferencesGroup::builder()
        .title(tr("Animated backgrounds"))
        .build();
    animated_group.set_visible(false);
    let status = ndl::ActionRow::builder().title("").build();
    let stop = gtk::Button::with_label(tr("Stop"));
    stop.set_valign(gtk::Align::Center);
    stop.set_sensitive(false);
    status.add_suffix(&stop);
    let pause_labels: Vec<&str> = PAUSE_CHOICES.iter().map(|l| tr(l)).collect();
    let pause = ndl::ComboRow::builder()
        .title(tr("Pause automatically"))
        .model(&gtk::StringList::new(&pause_labels))
        .build();
    pause.set_sensitive(false);
    status.add_prefix(&gtk::Image::from_icon_name("video-x-generic-symbolic"));
    animated_group.add(&status);
    animated_group.add(&pause);
    content.append(&animated_group);
    let animated = Gallery::new();
    let animated_box = column();
    animated_box.set_margin_top(0);
    animated_box.set_margin_bottom(0);
    animated_box.set_margin_start(0);
    animated_box.set_margin_end(0);
    animated_box.append(&animated.flow);
    animated_box.set_visible(false);
    content.append(&animated_box);

    let loading = Rc::new(std::cell::Cell::new(true));
    let state: Rc<RefCell<PlayerState>> = Rc::default();

    let refresh = {
        let (proxy, state, status, stop, pause, animated, loading) = (
            proxy.clone(),
            state.clone(),
            status.clone(),
            stop.clone(),
            pause.clone(),
            animated.clone(),
            loading.clone(),
        );
        Rc::new(move || {
            let Some(service) = proxy.borrow().clone() else {
                return;
            };
            let (state, status, stop, pause, animated, loading) = (
                state.clone(),
                status.clone(),
                stop.clone(),
                pause.clone(),
                animated.clone(),
                loading.clone(),
            );
            glib::spawn_future_local(async move {
                let Some(json) = ask(&service, "GetWallpaperState", None).await else {
                    return;
                };
                let Ok(now) = PlayerState::parse(&json) else {
                    return;
                };
                loading.set(true);
                status.set_title(&now.message);
                stop.set_sensitive(now.requested);
                pause.set_sensitive(now.options.pause.is_some());
                if let Some(p) = now.options.pause {
                    pause.set_selected(p);
                }
                animated.mark(if now.requested { &now.path } else { "" });
                loading.set(false);
                *state.borrow_mut() = now;
            });
        })
    };

    // Connects to the service, then fills the animated gallery.
    {
        let (proxy, animated, animated_group, animated_box, refresh, toasts) = (
            proxy.clone(),
            animated.clone(),
            animated_group.clone(),
            animated_box.clone(),
            refresh.clone(),
            toasts.clone(),
        );
        glib::spawn_future_local(async move {
            let Ok(service) = gio::DBusProxy::for_bus_future(
                gio::BusType::Session,
                gio::DBusProxyFlags::DO_NOT_LOAD_PROPERTIES,
                None,
                BUS,
                PATH,
                INTERFACE,
            )
            .await
            else {
                return;
            };
            *proxy.borrow_mut() = Some(service.clone());
            let Some(json) = ask(&service, "GetWallpaperState", None).await else {
                return;
            };
            if !PlayerState::parse(&json).is_ok_and(|s| s.available) {
                return;
            }
            animated_group.set_visible(true);
            animated_box.set_visible(true);
            // The service makes the first frame of a video you imported a little after it
            // lists it, so look again until every video has one (or the wait is over).
            for attempt in 0..POLLS {
                let pending = fill(&service, &animated, &toasts, &refresh).await;
                if !pending || attempt + 1 == POLLS {
                    break;
                }
                glib::timeout_future_seconds(POLL_SECONDS).await;
            }
        });
    }
    {
        let (proxy, refresh) = (proxy.clone(), refresh.clone());
        stop.connect_clicked(move |_| {
            let (proxy, refresh) = (proxy.borrow().clone(), refresh.clone());
            if let Some(proxy) = proxy {
                glib::spawn_future_local(async move {
                    let _ = proxy
                        .call_future("StopWallpaper", None, gio::DBusCallFlags::NONE, 10_000)
                        .await;
                    refresh();
                });
            }
        });
    }
    {
        let (proxy, loading) = (proxy.clone(), loading.clone());
        pause.connect_selected_notify(move |row| {
            if loading.get() {
                return;
            }
            let Some(proxy) = proxy.borrow().clone() else {
                return;
            };
            let value = row.selected() as i32;
            glib::spawn_future_local(async move {
                let args = ("pause-on-maximize-or-fullscreen", value).to_variant();
                let _ = proxy
                    .call_future(
                        "SetWallpaperOption",
                        Some(&args),
                        gio::DBusCallFlags::NONE,
                        4_000,
                    )
                    .await;
            });
        });
    }
    // The service says when the wallpaper changes from somewhere else.
    {
        let (proxy, refresh) = (proxy.clone(), refresh.clone());
        let weak = content.downgrade();
        glib::timeout_add_seconds_local(5, move || {
            let Some(content) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if content.is_mapped() && proxy.borrow().is_some() {
                refresh();
            }
            glib::ControlFlow::Continue
        });
    }
    content
}

/// How many times, and how far apart, the gallery asks again while pictures are being made.
const POLLS: u32 = 15;
const POLL_SECONDS: u32 = 4;

/// Fills the animated gallery from the service; says whether some video has no picture yet.
async fn fill(
    service: &gio::DBusProxy,
    animated: &Rc<Gallery>,
    toasts: &ndl::ToastOverlay,
    refresh: &Rc<impl Fn() + 'static>,
) -> bool {
    let catalog = match ask(service, "GetWallpapers", None).await {
        Some(text) => wallpapers::parse_catalog(&text).unwrap_or_default(),
        None => Vec::new(),
    };
    let pending = catalog.iter().any(Wallpaper::awaits_preview);
    animated.clear();
    for item in catalog {
        add_animated(animated, service, toasts, refresh, item);
    }
    refresh();
    pending
}

fn add_animated(
    gallery: &Rc<Gallery>,
    service: &gio::DBusProxy,
    toasts: &ndl::ToastOverlay,
    refresh: &Rc<impl Fn() + 'static>,
    item: Wallpaper,
) {
    let (service, toasts, refresh) = (service.clone(), toasts.clone(), refresh.clone());
    let path = item.path.clone();
    let tooltip = item.details();
    let preview = PathBuf::from(&item.preview);
    let video = Some(PathBuf::from(&item.path));
    gallery.add(&item.path, preview, video, &tooltip, move || {
        let (service, toasts, refresh, path) = (
            service.clone(),
            toasts.clone(),
            refresh.clone(),
            path.clone(),
        );
        glib::spawn_future_local(async move {
            let args = (path.as_str(),).to_variant();
            // Setting one can take a while: the service prepares the video first.
            match service
                .call_future(
                    "SetWallpaper",
                    Some(&args),
                    gio::DBusCallFlags::NONE,
                    60_000,
                )
                .await
            {
                Ok(_) => refresh(),
                Err(mut error) => {
                    gio::DBusError::strip_remote_error(&mut error);
                    toasts.add_toast(ndl::Toast::new(error.message()));
                }
            }
        });
    });
}
