from __future__ import annotations

import unittest
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHELL = ROOT / "shell"


class StaticInvariantTests(unittest.TestCase):
    def test_phosphor_font_covers_shell_icon_glyphs(self) -> None:
        manifest = json.loads(
            (SHELL / "assets/icons/phosphor-compat-manifest.json").read_text(encoding="utf-8")
        )
        font = SHELL / "assets/fonts/NodalixPhosphorCompat.ttf"
        self.assertTrue(font.is_file())
        self.assertGreater(font.stat().st_size, 100_000)
        self.assertTrue((SHELL / "assets/fonts/LICENSE-phosphor").is_file())
        for qml in SHELL.rglob("*.qml"):
            text = qml.read_text(encoding="utf-8")
            self.assertNotIn("font.family: ThemeManager.fontFamily", text, str(qml))
            for char in text:
                point = ord(char)
                if 0xF0000 <= point <= 0xFFFFD:
                    self.assertIn(f"U+{point:X}", manifest, str(qml))
        theme = (SHELL / "theme/ThemeManager.qml").read_text(encoding="utf-8")
        self.assertIn('iconFontFamily: "Nodalix Phosphor Compat"', theme)
        self.assertIn("function fontFor(value)", theme)
        self.assertIn("content.codePointAt(index)", theme)
        self.assertNotIn("Array.from", theme)

    def test_workspace_monitor_assignment_refreshes_bar_visibility(self) -> None:
        workspaces = (SHELL / "widgets/bar/Workspaces.qml").read_text(encoding="utf-8")
        self.assertIn("function onMonitorChanged()", workspaces)
        self.assertIn("monitorName === root.barScreen.name", workspaces)
        self.assertIn('startsWith("special:")', workspaces)
        self.assertNotIn("modelData.id >= 0 : true", workspaces)
        self.assertNotIn("modelData.monitor === root.hyprMonitor", workspaces)

    def test_colloid_icon_is_shell_icon_compatibility_wrapper(self) -> None:
        component = (SHELL / "widgets" / "bar" / "ColloidIcon.qml").read_text(encoding="utf-8")
        self.assertIn("ShellIcon {", component)
        self.assertNotIn("assets/icons/colloid-bold/", component)
        self.assertNotIn("ColorOverlay", component)

        shell_icon = SHELL / "widgets" / "bar" / "ShellIcon.qml"
        self.assertTrue(shell_icon.is_file())

    def test_bar_separators_are_removed_without_touching_panel_dividers(self) -> None:
        for name in ("BarLeft", "BarCenter", "BarRight"):
            bar = (SHELL / "bar" / f"{name}.qml").read_text(encoding="utf-8")
            self.assertNotIn("BarSeparator", bar)
            self.assertNotIn("width: 1; height: 14", bar)
        clock = (SHELL / "widgets/bar/Clock.qml").read_text(encoding="utf-8")
        self.assertNotIn("width: 1; height: 12", clock)

    def test_font_change_runs_packaged_script_through_shell_and_finishes(self) -> None:
        font_service = (SHELL / "services/FontService.qml").read_text(encoding="utf-8")
        self.assertIn('["/bin/sh", Paths.configDir + "/scripts/nodalix-set-font.sh", fontFamily]', font_service)
        self.assertIn("property Timer _deadline: Timer", font_service)
        self.assertIn("root.busy = false", font_service)

    def test_colloid_theme_is_packaged_and_defaulted_once(self) -> None:
        definition = json.loads((ROOT / "release/components.json").read_text(encoding="utf-8"))
        self.assertTrue(any(
            component["package"] == "nodalix-colloid-icons" and component["required"]
            for component in definition["components"]
        ))
        pkgbuild = (ROOT / "packaging/nodalix-colloid-icons/PKGBUILD").read_text(encoding="utf-8")
        self.assertIn("#commit=ceac6608ecd0e40025cbc2ebbd32bf0e0f4ebc6a", pkgbuild)
        self.assertIn("install.sh -d \"$pkgdir/usr/share/icons\" -t teal -b", pkgbuild)
        wrapper = (ROOT / "packaging/nodalix-shell/nodalix-shell").read_text(encoding="utf-8")
        self.assertIn(".colloid-icon-theme-v1", wrapper)
        self.assertIn("[ -d /usr/share/icons/Colloid-Teal ]", wrapper)
        self.assertIn("gsettings set org.gnome.desktop.interface icon-theme Colloid-Teal", wrapper)

    def test_greeter_hardening_and_cursor_theme_are_release_components(self) -> None:
        definition = json.loads((ROOT / "release/components.json").read_text(encoding="utf-8"))
        self.assertTrue(any(
            component["package"] == "nodalix-cursor-theme" and component["required"]
            for component in definition["components"]
        ))

        cursor_pkg = (ROOT / "packaging/nodalix-cursor-theme/PKGBUILD").read_text(encoding="utf-8")
        self.assertIn("Bibata_Cursor/releases/download/v2.0.7/Bibata.tar.xz", cursor_pkg)
        self.assertIn("172e33c4ae415278384dcecc7d1a9b7a024266bc944bc751fd86532be1cc6251", cursor_pkg)
        self.assertIn("Name=Nodalix", cursor_pkg)

        greetd = (ROOT / "iso/overlay/etc/greetd/config.toml").read_text(encoding="utf-8")
        packaged_greetd = (ROOT / "packaging/nodalix-greeter-theme/greetd-config.toml").read_text(encoding="utf-8")
        self.assertEqual(greetd, packaged_greetd)
        self.assertIn("/usr/lib/nodalix/nodalix-greeter-session", greetd)
        self.assertNotIn("cage -s -- regreet", greetd)

        greeter_hypr = (ROOT / "iso/overlay/etc/greetd/hyprland.lua").read_text(encoding="utf-8")
        packaged_hypr = (ROOT / "packaging/nodalix-greeter-theme/hyprland.lua").read_text(encoding="utf-8")
        self.assertEqual(greeter_hypr, packaged_hypr)
        self.assertIn('hl.env("XCURSOR_THEME", "Nodalix")', greeter_hypr)
        self.assertIn("enable_stdout_logs = true", greeter_hypr)
        self.assertIn('output = ""', greeter_hypr)

        regreet = (ROOT / "iso/overlay/etc/greetd/regreet.toml").read_text(encoding="utf-8")
        packaged_regreet = (ROOT / "packaging/nodalix-greeter-theme/regreet.toml").read_text(encoding="utf-8")
        self.assertEqual(regreet, packaged_regreet)
        self.assertIn('cursor_theme_name = "Nodalix"', regreet)

        wrapper = (ROOT / "packaging/nodalix-greeter-theme/nodalix-greeter-session").read_text(encoding="utf-8")
        self.assertIn('log_file="$log_dir/compositor-$timestamp-', wrapper)
        self.assertIn("tail -n +11", wrapper)
        self.assertIn("start-hyprland -- -c /etc/greetd/hyprland.lua", wrapper)

        packages = (ROOT / "iso/installer/packages.txt").read_text(encoding="utf-8").split()
        self.assertNotIn("cage", packages)

    def test_quickshell_does_not_hardcode_binding_service_actions(self) -> None:
        text = (SHELL / "hypr" / "quickshell.lua").read_text(encoding="utf-8")
        self.assertIn("binds.generated.lua", text)
        self.assertNotIn("SUPER + SPACE", text)
        self.assertNotIn("SUPER+SPACE", text)
        self.assertNotRegex(text, r'hl\.bind\([^)]*launcher toggle')
        for action in ("settings toggle", "lock lock", "tools toggle", "scratchpad toggle"):
            self.assertNotIn(f"ipc call {action}", text)

    def test_binding_service_keeps_launcher_default(self) -> None:
        text = (SHELL / "services" / "BindingService.qml").read_text(encoding="utf-8")
        self.assertIn('key: "launcher"', text)
        self.assertIn('k === "launcher" ? "SUPER + SPACE"', text)
        self.assertIn("binds.generated.lua", text)

    def test_local_canonical_overlay_sequence_is_preserved(self) -> None:
        main = (SHELL / "MainWindow.qml").read_text(encoding="utf-8")
        self.assertIn("property int _overlaySequence: 100", main)
        self.assertIn('function _raiseOverlay(kind)', main)
        self.assertIn('root._raiseOverlay("notification")', main)
        self.assertIn('root._raiseOverlay("popout")', main)
        self.assertIn('root._raiseOverlay("launcher")', main)
        self.assertIn("z:      root._notificationStackZ", main)
        self.assertIn("z:       root._popoutStackZ", main)
        self.assertIn("z: root._launcherStackZ", main)
        self.assertFalse((SHELL / "BarOverlay.qml").exists())
        self.assertFalse((SHELL / "services" / "OverlayManager.qml").exists())

    def test_popout_hover_leave_timer(self) -> None:
        text = (SHELL / "services" / "PopoutService.qml").read_text(encoding="utf-8")
        self.assertRegex(text, r"interval:\s*600")
        self.assertIn("pinned || widgetHovered || panelHovered", text)
        self.assertIn("if (!root.widgetHovered && !root.panelHovered) root.close()", text)

    def test_system_info_is_locale_independent(self) -> None:
        text = (SHELL / "services" / "SystemControlService.qml").read_text(encoding="utf-8")
        self.assertIn("export LC_ALL=C", text)
        self.assertIn("/proc/cpuinfo", text)
        self.assertIn("lspci", text)
        self.assertIn("Display controller", text)

    def test_gpu_name_is_normalized_for_compact_system_info(self) -> None:
        text = (SHELL / "services" / "SystemControlService.qml").read_text(encoding="utf-8")
        self.assertIn("function normalizedGpuName(value)", text)
        self.assertIn('name.split("/", 1)[0].trim() + " Series"', text)
        self.assertIn("root.normalizedGpuName(data.gpu)", text)

    def test_update_auth_prompt_is_inline(self) -> None:
        text = (SHELL / "panels" / "Settings.qml").read_text(encoding="utf-8")
        self.assertIn("component UpdateAuthPrompt", text)
        self.assertIn('UpdateAuthPrompt { action: "all" }', text)
        self.assertIn("UpdateAuthPrompt { action: modelData.key }", text)
        self.assertNotIn("id: _updatePassword", text)

    def test_shell_reload_does_not_kill_launched_apps(self) -> None:
        unit = (ROOT / "packaging/nodalix-shell/nodalix-shell.service").read_text(encoding="utf-8")
        self.assertIn("KillMode=process", unit)
        self.assertNotIn("KillMode=mixed", unit)

    def test_session_services_are_not_enabled_globally_for_greeter(self) -> None:
        shell_pkgbuild = (ROOT / "packaging/nodalix-shell/PKGBUILD").read_text(encoding="utf-8")
        phone_pkgbuild = (ROOT / "packaging/nodalix-phone-link/PKGBUILD").read_text(encoding="utf-8")
        shell_unit = (ROOT / "packaging/nodalix-shell/nodalix-shell.service").read_text(encoding="utf-8")
        accent_unit = (ROOT / "packaging/nodalix-shell/nodalix-app-accent.path").read_text(encoding="utf-8")
        phone_unit = (ROOT / "phone-link/systemd/nodalix-phone-link.service").read_text(encoding="utf-8")
        self.assertNotIn("/etc/systemd/user/default.target.wants", shell_pkgbuild)
        self.assertNotIn("/etc/systemd/user/default.target.wants", phone_pkgbuild)
        for unit in (shell_unit, accent_unit, phone_unit):
            self.assertIn("WantedBy=graphical-session.target", unit)
            self.assertNotIn("WantedBy=default.target", unit)

    def test_icloud_drive_packaging_is_self_contained(self) -> None:
        pkgbuild = (ROOT / "packaging/nodalix-shell/PKGBUILD").read_text(encoding="utf-8")
        service = (SHELL / "systemd/nodalix-icloud-drive.service").read_text(encoding="utf-8")
        helper = SHELL / "scripts/nodalix-icloud-apply-icon.fish"

        self.assertIn("rclone", pkgbuild)
        self.assertIn("fuse3", pkgbuild)
        self.assertIn("*nodalix-icloud-drive.service)", pkgbuild)
        self.assertIn("nodalix-icloud-apply-icon.fish", pkgbuild)

        self.assertTrue(helper.is_file())
        self.assertIn("ConditionPathExists=%h/.config/rclone/rclone.conf", service)
        self.assertIn("ExecStart=/usr/bin/rclone mount icloud:", service)
        self.assertIn("ExecStartPost=/usr/bin/nodalix-icloud-apply-icon", service)
        self.assertIn("ExecStop=/usr/bin/fusermount3", service)


    def test_avatar_dialog_releases_exclusive_settings_focus(self) -> None:
        settings = (SHELL / "panels" / "Settings.qml").read_text(encoding="utf-8")
        self.assertIn("function _chooseAvatarImage()", settings)
        self.assertIn("SettingsUi.hide()", settings)
        self.assertIn('"zenity", "--file-selection"', settings)
        self.assertIn("onExited: function(code)", settings)
        self.assertIn("root._restoreAfterAvatarDialog()", settings)
        self.assertIn('onClicked: root._chooseAvatarImage()', settings)
        self.assertNotIn('onClicked: _avatarDialog.open()', settings)
        self.assertNotIn("FileDialog {", settings)

    def test_first_state_migration_replaces_partial_validation_state(self) -> None:
        launcher = (ROOT / "packaging/nodalix-shell/nodalix-shell").read_text(encoding="utf-8")
        self.assertIn('cp -a "$old_state/." "$new_state/"', launcher)
        self.assertNotIn("--no-clobber", launcher)

    def test_dashboard_uses_local_canonical_popout_service(self) -> None:
        dashboard = (SHELL / "panels/Dashboard.qml").read_text(encoding="utf-8")
        self.assertNotIn("OverlayManager", dashboard)
        self.assertIn('PopoutService.currentName === "dashboard"', dashboard)

    def test_legacy_migration_checks_pacman_ownership(self) -> None:
        migration = (ROOT / "updater" / "migrations" / "to_0_2_0.py").read_text(encoding="utf-8")
        registry = (ROOT / "updater" / "migrations" / "registry.py").read_text(encoding="utf-8")
        self.assertIn("has_unowned_legacy_files", migration)
        self.assertIn("pacman", registry)
        self.assertIn("-Qo", registry)
        self.assertIn("/var/lib/nodalix-updater", ROOT.joinpath("updater/nodalix-updater").read_text(encoding="utf-8"))


    def test_calendar_bridge_is_packaged_and_runs_after_vdirsyncer(self) -> None:
        pkgbuild = (ROOT / "packaging/nodalix-shell/PKGBUILD").read_text(encoding="utf-8")
        service = (SHELL / "systemd/nodalix-icloud-sync.service").read_text(encoding="utf-8")
        bridge = SHELL / "scripts/nodalix-calendar-bridge.py"
        text = bridge.read_text(encoding="utf-8")

        self.assertTrue(bridge.is_file())
        self.assertIn("nodalix-calendar-bridge.py", pkgbuild)
        self.assertIn("python-gobject", pkgbuild)
        self.assertIn("evolution-data-server", pkgbuild)

        export = "ExecStart=/usr/bin/nodalix-calendar-bridge --export"
        sync = "ExecStart=/usr/bin/vdirsyncer sync"
        import_ = "ExecStart=/usr/bin/nodalix-calendar-bridge --import"

        self.assertIn(export, service)
        self.assertIn(sync, service)
        self.assertIn(import_, service)
        self.assertLess(service.index(export), service.index(sync))
        self.assertLess(service.index(sync), service.index(import_))

        self.assertIn("nodalix-calendar", text)
        self.assertIn("EDataServer.Source.new_with_uid", text)
        self.assertIn("get_object_list_as_comps_sync", text)
        self.assertIn("ECal.ObjModType.THIS", text)
        self.assertIn("X-NODALIX-SOURCE-FILE:", text)


    def test_shell_sliders_use_shared_thumbless_component(self) -> None:
        slider = (SHELL.parent / "shell/components/NodalixSlider.qml").read_text(encoding="utf-8")
        settings = (SHELL / "panels/Settings.qml").read_text(encoding="utf-8")
        dashboard = (SHELL / "panels/Dashboard.qml").read_text(encoding="utf-8")
        audio = (SHELL / "panels/AudioPanel.qml").read_text(encoding="utf-8")

        self.assertIn("readonly property int trackHeight: 20", slider)
        self.assertNotIn("thumb", slider.lower())

        self.assertGreaterEqual(settings.count("NodalixSlider {"), 4)
        self.assertIn("NodalixSlider {", dashboard)
        self.assertIn("NodalixSlider {", audio)

        self.assertNotIn("component VertSlider:", dashboard)
        self.assertNotIn("component DashSlider:", dashboard)
        self.assertNotIn("id: _wpIvTrack", settings)
        self.assertNotIn("id: liveTrack", settings)
        self.assertNotIn("id: hdrTrack", settings)

        for source in (settings, dashboard, audio):
            self.assertNotIn("_thumbR", source)
            self.assertNotIn("thumbCY", source)


    def test_phone_link_reserves_bluez_hfp_for_ofono(self) -> None:
        dropin = (
            ROOT
            / "phone-link/systemd/50-nodalix-phone-link-bluez-hfp.conf"
        ).read_text(encoding="utf-8")
        pkgbuild = (
            ROOT / "packaging/nodalix-phone-link/PKGBUILD"
        ).read_text(encoding="utf-8")
        install = (
            ROOT
            / "packaging/nodalix-phone-link/nodalix-phone-link.install"
        ).read_text(encoding="utf-8")
        wireplumber = (
            ROOT / "phone-link/wireplumber/51-nodalix-phone-link.conf"
        ).read_text(encoding="utf-8")

        self.assertIn(
            "ExecStart=/usr/lib/bluetooth/bluetoothd --noplugin=hfp",
            dropin,
        )
        self.assertIn(
            "bluetooth.service.d/50-nodalix-phone-link-bluez-hfp.conf",
            pkgbuild,
        )
        self.assertIn("--noplugin=hfp", install)
        self.assertIn(
            '"bluez5.hfphsp-backend" = "ofono"',
            wireplumber,
        )
        self.assertIn('"hfp_hf"', wireplumber)
        self.assertIn('"a2dp_source"', wireplumber)
        self.assertNotIn('"a2dp_sink"', wireplumber)


    def test_live_phone_calls_are_pinned_and_resident(self) -> None:
        service = (
            SHELL / "services/NotificationService.qml"
        ).read_text(encoding="utf-8")
        panel = (
            SHELL / "panels/NotificationCenter.qml"
        ).read_text(encoding="utf-8")

        self.assertIn("function _pinLiveCallsFirst(items)", service)
        self.assertIn("function _rebuildNotificationViews()", service)
        self.assertIn(
            "notif.actionsChanged.connect(root._rebuildNotificationViews)",
            service,
        )
        self.assertIn(
            "notif.hintsChanged.connect(root._rebuildNotificationViews)",
            service,
        )
        self.assertIn(
            "centerNotifs = _pinLiveCallsFirst(center)",
            service,
        )
        self.assertIn("function _callActionIds(notif)", service)
        self.assertIn('"x-nodalix-phone-call"', service)
        self.assertIn('"x-nodalix-call-path"', service)
        self.assertIn("function invokeNotificationAction(notif, action)", service)
        self.assertIn('"com.gabriel.iphonebridge.Calls1"', service)
        self.assertIn('"AnswerCall"', service)
        self.assertIn('"HangupCall"', service)
        self.assertIn('ids.indexOf("answer") >= 0', service)
        self.assertIn('ids.indexOf("hangup") >= 0', service)
        self.assertNotIn(
            "notif.urgency === NotificationUrgency.Critical",
            service,
        )
        self.assertIn("readonly property var centerNotifs:", service)
        self.assertIn("return _pinLiveCallsFirst(a)", service)
        self.assertIn("if (_isLiveCall(notif)) return", service)
        self.assertIn(
            "const calls = notifList.filter(n => _isLiveCall(n))",
            service,
        )
        self.assertIn(
            "? NotificationService.toastNotifs",
            panel,
        )
        self.assertIn(
            ": NotificationService.centerNotifs",
            panel,
        )
        self.assertIn("required property var modelData", panel)
        self.assertIn("readonly property var notif: modelData", panel)
        self.assertIn(
            "visible: !NotificationService._isLiveCall(_notifDelegate.notif)",
            panel,
        )
        self.assertIn(
            "NotificationService.invokeNotificationAction(",
            panel,
        )
        self.assertIn(
            "&& !_notifDelegate._hasActions",
            panel,
        )

        libnotify = (
            ROOT / "phone-link/src/iphonebridge/sinks/libnotify.py"
        ).read_text(encoding="utf-8")
        self.assertIn('"x-nodalix-phone-call": dbus.Boolean(True)', libnotify)
        self.assertIn('"x-nodalix-call-path": dbus.String(event.call_path)', libnotify)

        migration = (
            ROOT / "phone-link/systemd/migrate-user"
        ).read_text(encoding="utf-8")
        self.assertIn("51-bluez-hfp-hf.conf", migration)
        self.assertIn("99-nodalix-bluetooth-audio.conf", migration)
        self.assertIn(".bak.nodalix-0.2.5.", migration)

    def test_phone_contacts_are_mirrored_to_gnome_contacts(self) -> None:
        bridge = (
            ROOT / "phone-link/src/iphonebridge/eds_contacts.py"
        ).read_text(encoding="utf-8")
        contacts = (
            ROOT / "phone-link/src/iphonebridge/contacts.py"
        ).read_text(encoding="utf-8")
        pkgbuild = (
            ROOT / "packaging/nodalix-phone-link/PKGBUILD"
        ).read_text(encoding="utf-8")

        self.assertIn('SOURCE_UID = "nodalix-iphone-contacts"', bridge)
        self.assertIn('SOURCE_NAME = "iPhone (Nodalix)"', bridge)
        self.assertIn("SOURCE_EXTENSION_ADDRESS_BOOK", bridge)
        self.assertIn("EBook.BookClient.connect_sync", bridge)
        self.assertIn("new_from_vcard_with_uid", bridge)
        self.assertIn("sync_gnome_contacts(blob)", contacts)
        self.assertIn("'evolution-data-server'", pkgbuild)
        self.assertIn("'ofono'", pkgbuild)
        self.assertIn("pkgver=0.2.5", pkgbuild)


if __name__ == "__main__":
    unittest.main()
