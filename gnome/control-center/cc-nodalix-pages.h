/* SPDX-License-Identifier: GPL-3.0-or-later */
#pragma once
#include <adwaita.h>
#define CC_TYPE_NODALIX_UPDATES_PAGE (cc_nodalix_updates_page_get_type())
G_DECLARE_FINAL_TYPE(CcNodalixUpdatesPage, cc_nodalix_updates_page, CC, NODALIX_UPDATES_PAGE, AdwNavigationPage)
#define CC_TYPE_NODALIX_WALLPAPERS_PAGE (cc_nodalix_wallpapers_page_get_type())
G_DECLARE_FINAL_TYPE(CcNodalixWallpapersPage, cc_nodalix_wallpapers_page, CC, NODALIX_WALLPAPERS_PAGE, AdwNavigationPage)

#define CC_TYPE_NODALIX_LOCALSEND_PAGE (cc_nodalix_localsend_page_get_type())
G_DECLARE_FINAL_TYPE(CcNodalixLocalSendPage,cc_nodalix_localsend_page,CC,NODALIX_LOCALSEND_PAGE,AdwNavigationPage)
