/* SPDX-License-Identifier: GPL-3.0-or-later */
#pragma once
#include <adwaita.h>
#define CC_TYPE_NODALIX_BACKGROUND_CHOOSER (cc_nodalix_background_chooser_get_type())
G_DECLARE_FINAL_TYPE (CcNodalixBackgroundChooser, cc_nodalix_background_chooser, CC, NODALIX_BACKGROUND_CHOOSER, GtkBox)

#define CC_TYPE_NODALIX_WALLPAPER_PAGE (cc_nodalix_wallpaper_page_get_type ())
G_DECLARE_FINAL_TYPE (CcNodalixWallpaperPage, cc_nodalix_wallpaper_page, CC, NODALIX_WALLPAPER_PAGE, AdwNavigationPage)
