//! Nodalix System Monitor.

mod disks;
mod graph;
mod i18n;
mod process_object;
mod processes;
mod resources;
mod sampler;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use monitor_core::procs::Signal;
use ndl::gtk::{self, gio, glib, prelude::*};
use nodalia_gtk as ndl;

use crate::disks::DiskView;
use crate::i18n::tr;
use crate::processes::ProcessView;
use crate::resources::ResourceView;
use crate::sampler::Sampler;

const APP_ID: &str = "os.nodalix.Monitor";
const STYLE: &str = include_str!("style.css");

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| {
        ndl::init();
        install_style();
    });
    app.connect_activate(build_window);
    app.run()
}

fn install_style() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_string(STYLE);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 50,
    );
}

/// A pill the user picks a view with.
fn chip(label: &str, group: Option<&gtk::ToggleButton>) -> gtk::ToggleButton {
    let button = gtk::ToggleButton::with_label(label);
    button.add_css_class("ndl-chip");
    if let Some(group) = group {
        button.set_group(Some(group));
    }
    button
}

fn build_window(app: &gtk::Application) {
    if let Some(window) = app.active_window() {
        window.present();
        return;
    }
    let window = ndl::ApplicationWindow::new(app);
    window.set_title(Some(tr("System Monitor")));
    window.set_default_size(960, 680);
    window.set_icon_name(Some("utilities-system-monitor"));

    let sampler = Rc::new(RefCell::new(Sampler::default()));
    let first = sampler.borrow_mut().sample();
    let procs = ProcessView::new();
    let resources = ResourceView::new(first.cores.len());
    let disks = DiskView::new();
    procs.update(&first);

    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.add_named(&procs.widget, Some("processes"));
    stack.add_named(&resources.widget, Some("resources"));
    stack.add_named(&disks.widget, Some("disks"));

    let toasts = ndl::ToastOverlay::new();
    let toolbar = ndl::ToolbarView::new();
    let header = ndl::HeaderBar::new();

    let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let c_procs = chip(tr("Processes"), None);
    let c_res = chip(tr("Resources"), Some(&c_procs));
    let c_disks = chip(tr("File systems"), Some(&c_procs));
    c_procs.set_active(true);
    for (button, page) in [
        (&c_procs, "processes"),
        (&c_res, "resources"),
        (&c_disks, "disks"),
    ] {
        tabs.append(button);
        let stack = stack.clone();
        button.connect_toggled(move |b| {
            if b.is_active() {
                stack.set_visible_child_name(page);
            }
        });
    }
    header.set_title_widget(Some(&tabs));

    // Process actions live in the header so they sit next to the table they act on.
    let end = gtk::Button::with_label(tr("End process"));
    end.add_css_class("destructive-action");
    end.set_sensitive(false);
    let more = gtk::MenuButton::new();
    more.set_icon_name("view-more-symbolic");
    more.set_sensitive(false);
    header.pack_end(&more);
    header.pack_end(&end);

    let menu = gio::Menu::new();
    let section = gio::Menu::new();
    section.append_item(&ndl::menu_item(
        tr("Force quit"),
        "win.kill",
        "process-stop-symbolic",
    ));
    section.append_item(&ndl::menu_item(
        tr("Stop"),
        "win.stop",
        "media-playback-pause-symbolic",
    ));
    section.append_item(&ndl::menu_item(
        tr("Continue"),
        "win.cont",
        "media-playback-start-symbolic",
    ));
    menu.append_section(None, &section);
    let about = gio::Menu::new();
    about.append_item(&ndl::menu_item(
        tr("About"),
        "win.about",
        "help-about-symbolic",
    ));
    menu.append_section(None, &about);
    let popover = gtk::PopoverMenu::from_model(Some(&menu));
    ndl::menu_rows(&popover);
    more.set_popover(Some(&popover));
    // About needs no process: keep the button usable.
    more.set_sensitive(true);

    let signal_action = |name: &str, signal: Signal| {
        let action = gio::SimpleAction::new(name, None);
        let (procs, toasts) = (procs.clone(), toasts.clone());
        action.connect_activate(move |_, _| match procs.signal_selected(signal) {
            Ok(()) => {}
            Err(msg) if msg.is_empty() => {}
            Err(msg) => toasts.add_toast(ndl::Toast::new(&msg)),
        });
        window.add_action(&action);
        action
    };
    let a_kill = signal_action("kill", Signal::Kill);
    let a_stop = signal_action("stop", Signal::Stop);
    let a_cont = signal_action("cont", Signal::Continue);
    for a in [&a_kill, &a_stop, &a_cont] {
        a.set_enabled(false);
    }
    {
        let (procs, toasts) = (procs.clone(), toasts.clone());
        end.connect_clicked(move |_| match procs.signal_selected(Signal::Terminate) {
            Ok(()) => {}
            Err(msg) if msg.is_empty() => {}
            Err(msg) => toasts.add_toast(ndl::Toast::new(&msg)),
        });
    }
    {
        let end = end.clone();
        let actions = [a_kill, a_stop, a_cont];
        procs.connect_selection_changed(move |pid| {
            end.set_sensitive(pid.is_some());
            for a in &actions {
                a.set_enabled(pid.is_some());
            }
        });
    }
    {
        let about = gio::SimpleAction::new("about", None);
        let parent = window.clone();
        about.connect_activate(move |_, _| {
            ndl::AboutDialog::builder()
                .application_icon("utilities-system-monitor")
                .application_name(tr("System Monitor"))
                .developer_name("Nodalix")
                .version(env!("CARGO_PKG_VERSION"))
                .comments(tr("A system monitor for Nodalix."))
                .license_type(gtk::License::Gpl30)
                .build()
                .present(Some(&parent));
        });
        window.add_action(&about);
    }

    let search = gtk::ToggleButton::new();
    search.set_icon_name("edit-find-symbolic");
    search.set_tooltip_text(Some(tr("Search processes")));
    header.pack_start(&search);
    {
        let (procs, stack) = (procs.clone(), stack.clone());
        search.connect_toggled(move |b| {
            procs.search.set_visible(b.is_active());
            if b.is_active() {
                stack.set_visible_child_name("processes");
                procs.search.grab_focus();
            } else {
                procs.search.set_text("");
            }
        });
    }
    procs.search.set_visible(false);

    // For screenshots and tests: open on another page.
    match std::env::var("NODALIX_MONITOR_PAGE").as_deref() {
        Ok("resources") => c_res.set_active(true),
        Ok("disks") => c_disks.set_active(true),
        _ => {}
    }

    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&stack));
    toasts.set_child(Some(&toolbar));
    window.set_content(Some(&toasts));

    // One reading per second feeds every view; the disks are re-read every ten.
    let tick = Rc::new(RefCell::new(0u32));
    {
        let (sampler, procs, resources, disks, stack) = (
            sampler.clone(),
            procs.clone(),
            Rc::new(resources),
            Rc::new(disks),
            stack.clone(),
        );
        glib::timeout_add_local(Duration::from_secs(1), move || {
            let snapshot = sampler.borrow_mut().sample();
            // The graphs always take the reading, so their history has no gaps; the table is
            // the costly one and only updates while it is on screen.
            resources.update(&snapshot);
            match stack.visible_child_name().as_deref() {
                Some("disks") => {
                    let mut n = tick.borrow_mut();
                    *n += 1;
                    if *n % 10 == 0 {
                        disks.update();
                    }
                }
                Some("processes") => procs.update(&snapshot),
                _ => {}
            }
            glib::ControlFlow::Continue
        });
    }
    window.present();
}
