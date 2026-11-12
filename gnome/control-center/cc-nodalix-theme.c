/* cc-nodalix-theme.c
 *
 * Applies the Nodalia Design Language (NDL) on top of libadwaita.
 *
 * The sheets live in shell/ndl/ (see ndl/PROVENANCE.md). They are loaded below
 * the application priority so Settings' own style.css still wins, and above the
 * libadwaita theme so the palette, shapes and typography are NDL's. The colour
 * sheet is swapped when the system style or contrast changes.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#define G_LOG_DOMAIN "SETTINGS::NDL"

#include "config.h"

#include <adwaita.h>

#include "cc-nodalix-theme.h"

#define NDL_PREFIX "/org/gnome/Settings/ndl/"
/* Between GTK_STYLE_PROVIDER_PRIORITY_SETTINGS and _APPLICATION. */
#define NDL_PRIORITY (GTK_STYLE_PROVIDER_PRIORITY_APPLICATION - 50)

static GtkCssProvider *mode_provider = NULL;
static gboolean        installed     = FALSE;

static const char *
mode_sheet (AdwStyleManager *manager)
{
  gboolean dark = adw_style_manager_get_dark (manager);
  gboolean hc   = adw_style_manager_get_high_contrast (manager);

  if (hc)
    return dark ? "nodalia-dark-hc.css" : "nodalia-light-hc.css";
  return dark ? "nodalia-dark.css" : "nodalia-light.css";
}

static void
load_resource (GtkCssProvider *provider,
               const char     *name)
{
  g_autofree char *path = g_strconcat (NDL_PREFIX, name, NULL);

  gtk_css_provider_load_from_resource (provider, path);
}

static void
install_resource (GdkDisplay *display,
                  const char *name)
{
  GtkCssProvider *provider = gtk_css_provider_new ();

  load_resource (provider, name);
  gtk_style_context_add_provider_for_display (
      display, GTK_STYLE_PROVIDER (provider), NDL_PRIORITY);
  g_object_unref (provider);
}

static void
style_changed_cb (AdwStyleManager *manager,
                  GParamSpec      *pspec,
                  gpointer         user_data)
{
  if (mode_provider != NULL)
    load_resource (mode_provider, mode_sheet (manager));
}

void
cc_nodalix_theme_init (void)
{
  GdkDisplay      *display = NULL;
  AdwStyleManager *manager = NULL;
  const char      *opt_out = NULL;

  if (installed)
    return;

  opt_out = g_getenv ("NODALIX_NDL");
  if (opt_out != NULL && g_strcmp0 (opt_out, "0") == 0)
    return;

  display = gdk_display_get_default ();
  if (display == NULL)
    return;

  installed = TRUE;
  manager   = adw_style_manager_get_for_display (display);

  /* Order matters among providers of one priority: colours first, then the
   * metrics and the widget sheet that read them, then the Nodalix overrides. */
  mode_provider = gtk_css_provider_new ();
  load_resource (mode_provider, mode_sheet (manager));
  gtk_style_context_add_provider_for_display (
      display, GTK_STYLE_PROVIDER (mode_provider), NDL_PRIORITY);

  install_resource (display, "nodalia-metrics-regular.css");
  install_resource (display, "nodalia-gtk.css");
  install_resource (display, "nodalix-overrides.css");

  g_signal_connect (manager, "notify::dark", G_CALLBACK (style_changed_cb), NULL);
  g_signal_connect (manager, "notify::high-contrast", G_CALLBACK (style_changed_cb), NULL);
}
