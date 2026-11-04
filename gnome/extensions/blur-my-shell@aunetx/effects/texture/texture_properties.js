import GObject from 'gi://GObject';

import {clamp} from '../../conveniences/utils.js';
import {update_float_property} from '../../render/shader/uniforms.js';

function dimension(name) {
    return {
        configurable: true,
        get() {
            return this[`_${name}`];
        },
        set(value) {
            const size = clamp(value, 1, Number.MAX_SAFE_INTEGER, 1);
            update_float_property(this, name, size, -1e-6);
        },
    };
}

const properties = {
    opacity_factor: {
        configurable: true,
        get() {
            return this._opacity_factor;
        },
        set(value) {
            const opacity = clamp(value, 0, 1, this.constructor.default_params.opacity_factor);
            update_float_property(this, 'opacity_factor', opacity);
        },
    },
    width: dimension('width'),
    height: dimension('height'),
};

export function register_texture_effect(metadata, effectClass) {
    Object.defineProperties(effectClass.prototype, properties);
    return GObject.registerClass(metadata, effectClass);
}
