/* Exercise the production Gtk/libadwaita page without the Settings shell. */
#include "cc-nodalix-storage.c"
#include <glib/gstdio.h>

static gboolean
verify_loaded (gpointer data)
{
  CcNodalixStoragePage *page = data;
  if (page->busy || !page->data)
    return G_SOURCE_CONTINUE;
  GtkWidget *first = gtk_widget_get_first_child (GTK_WIDGET (page->items));
  g_assert_true (ADW_IS_ACTION_ROW (first));
  g_assert_cmpstr (adw_preferences_row_get_title (ADW_PREFERENCES_ROW (first)), ==, "Large fixture app");
  g_assert_nonnull (g_strstr_len (gtk_label_get_text (page->capacity), -1, "disponibles de"));
  adw_combo_row_set_selected (page->category, 1);
  first = gtk_widget_get_first_child (GTK_WIDGET (page->items));
  g_assert_cmpstr (adw_preferences_row_get_title (ADW_PREFERENCES_ROW (first)), ==, "Large test document.pdf");
  adw_combo_row_set_selected (page->category, 0);
  g_assert_true (g_file_set_contents (g_getenv ("STORAGE_UI_READY"), "ready", -1, NULL));
  return G_SOURCE_REMOVE;
}

static void
activate (GtkApplication *app)
{
  GtkWindow *window = GTK_WINDOW (adw_application_window_new (app));
  CcNodalixStoragePage *page = g_object_new (CC_TYPE_NODALIX_STORAGE_PAGE, NULL);
  AdwNavigationView *view = ADW_NAVIGATION_VIEW (adw_navigation_view_new ());
  adw_navigation_view_add (view, ADW_NAVIGATION_PAGE (page));
  gtk_window_set_title (window, "Storage fixture");
  gtk_window_set_default_size (window, 700, 620);
  adw_application_window_set_content (ADW_APPLICATION_WINDOW (window), GTK_WIDGET (view));
  gtk_window_present (window);
  g_signal_emit_by_name (page, "shown");
  g_timeout_add (50, verify_loaded, page);
}

int
main (int argc, char **argv)
{
  g_autoptr(AdwApplication) app = adw_application_new ("com.nodalix.StorageFixture", G_APPLICATION_NON_UNIQUE);
  g_signal_connect (app, "activate", G_CALLBACK (activate), NULL);
  return g_application_run (G_APPLICATION (app), argc, argv);
}
