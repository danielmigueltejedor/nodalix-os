/* Nodalia Design Language (NDL) for Nodalix Settings.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#pragma once

#include <glib.h>

G_BEGIN_DECLS

/* Installs the NDL style sheets for the default display and keeps the colour
 * mode (light, dark, high contrast) in sync with AdwStyleManager. Safe to call
 * more than once. NODALIX_NDL=0 in the environment skips it, leaving the stock
 * libadwaita look for debugging. */
void cc_nodalix_theme_init (void);

G_END_DECLS
