/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "cc-nodalix-release.h"

gchar *
cc_nodalix_release_version (const gchar *path)
{
  g_autofree gchar *contents = NULL;
  g_autoptr(GMatchInfo) match = NULL;
  g_autoptr(GRegex) pattern = g_regex_new (
      "^VERSION_ID=[\"']?([0-9]+\\.[0-9]+\\.[0-9]+(?:-[A-Za-z0-9.-]+)?)[\"']?\\r?$",
      G_REGEX_MULTILINE, 0, NULL);
  if (g_file_get_contents (path, &contents, NULL, NULL) &&
      g_regex_match (pattern, contents, 0, &match))
    return g_match_info_fetch (match, 1);
  return g_strdup ("No disponible");
}

static void
refresh_version (AdwActionRow *row)
{
  GFile *file = g_object_get_data (G_OBJECT (row), "nodalix-release-file");
  g_autofree gchar *path = g_file_get_path (file);
  g_autofree gchar *version = cc_nodalix_release_version (path);
  adw_action_row_set_subtitle (row, version);
}

static void
release_changed (GFileMonitor *monitor, GFile *file, GFile *other,
                 GFileMonitorEvent event, AdwActionRow *row)
{
  GFile *target = g_object_get_data (G_OBJECT (row), "nodalix-release-file");
  if ((file && g_file_equal (target, file)) ||
      (other && g_file_equal (target, other)))
    refresh_version (row);
}

static void
stop_monitor (gpointer data)
{
  g_file_monitor_cancel (G_FILE_MONITOR (data));
  g_object_unref (data);
}

void
cc_nodalix_release_watch (AdwActionRow *row, const gchar *path)
{
  g_autoptr(GFile) file = g_file_new_for_path (path);
  g_autoptr(GFile) parent = g_file_get_parent (file);
  GFileMonitor *monitor = g_file_monitor_directory (parent, G_FILE_MONITOR_WATCH_MOVES, NULL, NULL);
  g_object_set_data_full (G_OBJECT (row), "nodalix-release-file", g_object_ref (file), g_object_unref);
  refresh_version (row);
  if (monitor) {
    g_signal_connect_object (monitor, "changed", G_CALLBACK (release_changed), row, 0);
    g_object_set_data_full (G_OBJECT (row), "nodalix-release-monitor", monitor, stop_monitor);
  }
}
