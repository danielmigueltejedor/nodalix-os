//! Updates, a client of the session service `com.nodalix.Settings`, which keeps running when
//! Settings closes and does the privileged work through its own helper.

use std::cell::RefCell;
use std::rc::Rc;

use ajustes_core::updates::UpdateState;
use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use super::{column, scroller};
use crate::i18n::tr;

const BUS: &str = "com.nodalix.Settings";
const PATH: &str = "/com/nodalix/Settings";
const INTERFACE: &str = "com.nodalix.Settings1";

pub struct UpdatesPage {
    pub widget: gtk::ScrolledWindow,
    toasts: RefCell<Option<ndl::ToastOverlay>>,
    proxy: RefCell<Option<gio::DBusProxy>>,
    status: ndl::ActionRow,
    spinner: ndl::Spinner,
    check: gtk::Button,
    update_all: gtk::Button,
    banner: gtk::Box,
    banner_text: gtk::Label,
    sources: ndl::PreferencesGroup,
    source_rows: RefCell<Vec<ndl::ActionRow>>,
    log: gtk::TextView,
    log_expander: gtk::Expander,
    state: RefCell<UpdateState>,
}

impl UpdatesPage {
    pub fn new() -> Rc<Self> {
        let content = column();

        let banner_text = gtk::Label::new(Some(tr("Restart to finish the update")));
        banner_text.set_hexpand(true);
        banner_text.set_xalign(0.0);
        banner_text.set_wrap(true);
        let restart = gtk::Button::with_label(tr("Restart"));
        let banner = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        banner.add_css_class("settings-banner");
        banner.append(&banner_text);
        banner.append(&restart);
        banner.set_visible(false);
        content.append(&banner);

        let group = ndl::PreferencesGroup::builder().title(tr("Status")).build();
        let status = ndl::ActionRow::builder().title("…").build();
        let spinner = ndl::Spinner::new();
        spinner.set_visible(false);
        status.add_suffix(&spinner);
        group.add(&status);
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        buttons.set_margin_top(12);
        let check = gtk::Button::with_label(tr("Check for updates"));
        let update_all = gtk::Button::with_label(tr("Update everything"));
        update_all.add_css_class("suggested-action");
        buttons.append(&check);
        buttons.append(&update_all);
        content.append(&group);
        content.append(&buttons);

        let sources = ndl::PreferencesGroup::builder()
            .title(tr("Sources"))
            .build();
        content.append(&sources);

        let log = gtk::TextView::new();
        log.set_editable(false);
        log.set_cursor_visible(false);
        log.set_monospace(true);
        log.set_wrap_mode(gtk::WrapMode::WordChar);
        log.add_css_class("settings-log");
        let log_scroll = gtk::ScrolledWindow::builder()
            .child(&log)
            .min_content_height(220)
            .build();
        let log_expander = gtk::Expander::new(Some(tr("Show the update log")));
        log_expander.set_child(Some(&log_scroll));
        log_expander.set_visible(false);
        content.append(&log_expander);

        let page = Rc::new(Self {
            widget: scroller(&content),
            toasts: RefCell::new(None),
            proxy: RefCell::new(None),
            status,
            spinner,
            check: check.clone(),
            update_all: update_all.clone(),
            banner,
            banner_text,
            sources,
            source_rows: RefCell::new(Vec::new()),
            log,
            log_expander,
            state: RefCell::new(UpdateState::default()),
        });

        {
            let page = page.clone();
            check.connect_clicked(move |_| page.call("CheckUpdates", None));
        }
        {
            let page = page.clone();
            update_all
                .connect_clicked(move |_| page.call("StartUpdate", Some(("all",).to_variant())));
        }
        {
            let page = page.clone();
            restart.connect_clicked(move |_| page.call("Restart", None));
        }
        page.connect();
        page
    }

    /// Toasts show errors from the service; the window gives its overlay once it exists.
    pub fn set_toasts(&self, toasts: &ndl::ToastOverlay) {
        *self.toasts.borrow_mut() = Some(toasts.clone());
    }

    fn toast(&self, text: &str) {
        if let Some(t) = self.toasts.borrow().as_ref() {
            t.add_toast(ndl::Toast::new(text));
        }
    }

    fn connect(self: &Rc<Self>) {
        let page = self.clone();
        glib::spawn_future_local(async move {
            let proxy = gio::DBusProxy::for_bus_future(
                gio::BusType::Session,
                gio::DBusProxyFlags::DO_NOT_LOAD_PROPERTIES,
                None,
                BUS,
                PATH,
                INTERFACE,
            )
            .await;
            match proxy {
                Ok(proxy) => {
                    let weak = Rc::downgrade(&page);
                    // `connect_local`, not `connect_g_signal`: the handler touches widgets and is
                    // not `Send`; GDBus delivers on this thread's main context.
                    proxy.connect_local("g-signal", false, move |args| {
                        let signal: String = args.get(2)?.get().ok()?;
                        if signal == "Changed" {
                            weak.upgrade()?.refresh();
                        }
                        None
                    });
                    *page.proxy.borrow_mut() = Some(proxy);
                    page.refresh();
                }
                Err(error) => page.service_missing(&error.to_string()),
            }
        });
    }

    fn service_missing(&self, detail: &str) {
        eprintln!("com.nodalix.Settings: {detail}");
        self.status.set_title(tr("The update service is not running. It starts with your session; sign out and in again if this stays."));
        self.status.set_subtitle("");
        self.check.set_sensitive(false);
        self.update_all.set_sensitive(false);
    }

    fn refresh(self: &Rc<Self>) {
        let Some(proxy) = self.proxy.borrow().clone() else {
            return;
        };
        let page = self.clone();
        glib::spawn_future_local(async move {
            match proxy
                .call_future("GetUpdates", None, gio::DBusCallFlags::NONE, 10_000)
                .await
            {
                Ok(reply) => {
                    let json = reply
                        .child_value(0)
                        .str()
                        .map(str::to_owned)
                        .unwrap_or_default();
                    match UpdateState::parse(&json) {
                        Ok(state) => page.render(state),
                        Err(error) => page.service_missing(&error.to_string()),
                    }
                }
                Err(error) => page.service_missing(&error.to_string()),
            }
        });
    }

    fn call(self: &Rc<Self>, method: &'static str, args: Option<glib::Variant>) {
        let Some(proxy) = self.proxy.borrow().clone() else {
            return;
        };
        let page = self.clone();
        glib::spawn_future_local(async move {
            if let Err(error) = proxy
                .call_future(method, args.as_ref(), gio::DBusCallFlags::NONE, -1)
                .await
            {
                // The service words its own refusals ("Ya hay una operación en curso").
                let mut error = error;
                gio::DBusError::strip_remote_error(&mut error);
                page.toast(error.message());
            }
            page.refresh();
        });
    }

    fn render(self: &Rc<Self>, state: UpdateState) {
        self.status.set_title(&state.message);
        let mut detail = Vec::new();
        if !state.identity.is_empty() {
            detail.push(state.identity.clone());
        }
        if !state.checked_at.is_empty() {
            detail.push(format!("{} {}", tr("Last checked"), state.checked_at));
        }
        self.status.set_subtitle(&detail.join(" · "));
        let working = !state.idle();
        self.spinner.set_visible(working);
        self.check
            .set_sensitive(!working && !state.reboot_mandatory);
        self.update_all
            .set_sensitive(!working && !state.reboot_mandatory);

        self.banner.set_visible(state.reboot_required);
        if !state.reboot_reason.is_empty() {
            self.banner_text.set_text(&state.reboot_reason);
        }

        for row in self.source_rows.borrow_mut().drain(..) {
            self.sources.remove(&row);
        }
        for source in &state.sources {
            let row = ndl::ActionRow::builder()
                .title(&source.title)
                .subtitle(if source.available {
                    source.detail.as_str()
                } else {
                    tr("Not installed")
                })
                .build();
            if source.available {
                let button = gtk::Button::with_label(tr("Update"));
                button.set_valign(gtk::Align::Center);
                button.set_sensitive(!working && !state.reboot_mandatory);
                let (page, kind) = (self.clone(), source.kind.clone());
                button.connect_clicked(move |_| {
                    page.call("StartUpdate", Some((kind.as_str(),).to_variant()))
                });
                row.add_suffix(&button);
            }
            self.sources.add(&row);
            self.source_rows.borrow_mut().push(row);
        }

        if state.log.is_empty() {
            self.log_expander.set_visible(false);
        } else {
            self.log_expander.set_visible(true);
            if self.state.borrow().log != state.log {
                self.log.buffer().set_text(&state.log);
                // Follow the end of the log while an update runs.
                let end = self
                    .log
                    .buffer()
                    .create_mark(None, &self.log.buffer().end_iter(), false);
                self.log.scroll_to_mark(&end, 0.0, false, 0.0, 0.0);
            }
        }
        *self.state.borrow_mut() = state;
    }
}
