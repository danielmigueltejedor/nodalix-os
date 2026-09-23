// Native minimize compatibility for Hyprland builds without window.minimize.
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/desktop/view/Window.hpp>
#include <hyprland/src/desktop/state/WindowState.hpp>
#include <hyprland/src/event/EventBus.hpp>
#include <hyprland/src/protocols/XDGShell.hpp>
#include <hyprland/src/xwayland/XSurface.hpp>
#include <format>
#include <unordered_map>

namespace {
std::unordered_map<uintptr_t, CHyprSignalListener> listeners;
CHyprSignalListener opened, closed;
void request(PHLWINDOWREF weak) {
    const auto window = weak.lock();
    if (!window) return;
    std::optional<bool> minimized;
    if (window->m_xdgSurface && window->m_xdgSurface->m_toplevel)
        minimized = window->m_xdgSurface->m_toplevel->m_state.requestsMinimize;
    else if (window->m_xwaylandSurface) {
        minimized = window->m_xwaylandSurface->m_state.requestsMinimize;
        window->m_xwaylandSurface->m_state.requestsMinimize.reset();
    }
    if (!minimized.has_value()) return;
    const auto command = std::format("hl.dsp.exec_cmd(\"qs -c nodalix ipc call windows request {} 0x{:x}\")",
                                      *minimized ? "minimize" : "activate", reinterpret_cast<uintptr_t>(window.get()));
    HyprlandAPI::invokeHyprctlCommand("dispatch", command);
}
void attach(PHLWINDOW window) {
    const auto key = reinterpret_cast<uintptr_t>(window.get());
    if (listeners.contains(key)) return;
    PHLWINDOWREF weak = window;
    if (window->m_xdgSurface && window->m_xdgSurface->m_toplevel)
        listeners[key] = window->m_xdgSurface->m_toplevel->m_events.stateChanged.listen([weak] { request(weak); });
    else if (window->m_xwaylandSurface)
        listeners[key] = window->m_xwaylandSurface->m_events.stateChanged.listen([weak] { request(weak); });
}
}
APICALL EXPORT std::string PLUGIN_API_VERSION() { return HYPRLAND_API_VERSION; }
APICALL EXPORT PLUGIN_DESCRIPTION_INFO PLUGIN_INIT(HANDLE) {
    for (const auto& window : Desktop::windowState()->windows()) attach(window);
    opened = Event::bus()->m_events.window.open.listen([](PHLWINDOW window) { attach(window); });
    closed = Event::bus()->m_events.window.close.listen([](PHLWINDOW window) { listeners.erase(reinterpret_cast<uintptr_t>(window.get())); });
    return {"nodalix-window-controls", "Forward native minimize requests to the Nodalix dock", "Nodalix", "0.2.0"};
}
APICALL EXPORT void PLUGIN_EXIT() {
    opened.reset(); closed.reset(); listeners.clear();
}
