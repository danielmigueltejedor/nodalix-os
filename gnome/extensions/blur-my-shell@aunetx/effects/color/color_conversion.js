import GObject from 'gi://GObject';

import * as utils from '../../conveniences/utils.js';
import * as uniforms from '../../render/shader/uniforms.js';

const Shell = await utils.import_in_shell_only('gi://Shell');

export function color_conversion_effect(typeName, filename) {
    const defaultParams = {opacity_factor: 1};
    if (utils.IS_IN_PREFERENCES)
        return {default_params: defaultParams};

    const source = utils.get_shader_source(Shell, filename, import.meta.url);
    const metadata = {
        GTypeName: typeName,
        Properties: {
            opacity_factor: GObject.ParamSpec.double('opacity_factor', 'Opacity factor',
                'Opacity factor', GObject.ParamFlags.READWRITE, 0, 1, 1),
        },
    };

    return GObject.registerClass(metadata, class ColorConversionEffect extends utils.ShaderEffect {
        constructor(params) {
            super();
            utils.initialize_shader_effect(this, source);
            utils.setup_params(this, params);
        }

        static get default_params() {
            return defaultParams;
        }

        get opacity_factor() {
            return this._opacity_factor;
        }

        set opacity_factor(value) {
            const opacity = utils.clamp(value, 0, 1, defaultParams.opacity_factor);
            uniforms.update_float_property(this, 'opacity_factor', opacity);
        }

        vfunc_paint_target(paintNode, paintContext) {
            uniforms.upload_uniforms(this);
            super.vfunc_paint_target(paintNode, paintContext);
        }
    });
}
