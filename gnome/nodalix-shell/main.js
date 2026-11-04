// Built into the Nodalix-maintained GNOME Shell GResource, not an extension.
import {NodalixConnect} from './connect.js';

let started = false;
const components = [];

export function start({panel, sessionMode, notifyError}) {
    if (started || sessionMode.isGreeter || sessionMode.currentMode !== 'user')
        return;

    started = true;
    try {
        components.push(new NodalixConnect(panel.statusArea.quickSettings, notifyError));
    } catch (error) {
        console.error('Nodalix Shell: native Phone Link failed to initialize', error);
    }

    global.connect('shutdown', () => {
        for (const component of components.splice(0).reverse()) {
            try {
                component.destroy();
            } catch (error) {
                console.error('Nodalix Shell: cleanup failed', error);
            }
        }
        started = false;
    });
}
