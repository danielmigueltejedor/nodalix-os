// Built into the Nodalix-maintained GNOME Shell GResource, not an extension.
import {NodalixConnect} from './connect.js';
import {DesktopModules} from './modules.js';

let started = false;
const components = [];

export function start({panel, sessionMode, extensionManager, notifyError}) {
    if (started || sessionMode.isGreeter || sessionMode.currentMode !== 'user')
        return;

    started = true;
    try {
        components.push(new NodalixConnect(panel.statusArea.quickSettings, notifyError));
    } catch (error) {
        console.error('Nodalix Shell: native Phone Link failed to initialize', error);
    }

    try {
        components.push(new DesktopModules(sessionMode, extensionManager));
    } catch (error) {
        console.error('Nodalix Shell: desktop modules failed to initialize', error);
    }
}

export function stop() {
    for (const component of components.splice(0).reverse()) {
        try {
            component.destroy();
        } catch (error) {
            console.error('Nodalix Shell: cleanup failed', error);
        }
    }
    started = false;
}
