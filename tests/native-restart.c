/* Execute in an isolated test bus/display; Restart is served by a fake service. */
#include "../gnome/control-center/cc-nodalix-pages.c"
static GtkWindow *window;
static GMainLoop *loop;
static CcNodalixUpdatesPage *page;
static gboolean check_page(gpointer data) {
  g_assert_true(gtk_widget_get_visible(page->restart));
  g_assert_false(gtk_widget_get_sensitive(page->buttons[6]));
  g_assert_false(gtk_widget_get_sensitive(page->buttons[0]));
  g_signal_emit_by_name(page->restart,"clicked");
  return G_SOURCE_REMOVE;
}
static gboolean finish(gpointer data) {
  g_autoptr(GVariant) result=g_dbus_proxy_call_sync(page->proxy,"TestRestartCount",NULL,G_DBUS_CALL_FLAGS_NONE,2500,NULL,NULL);
  guint count=0;g_variant_get(result,"(u)",&count);g_assert_cmpuint(count,==,1);
  gtk_window_destroy(window);g_main_loop_quit(loop);
  return G_SOURCE_REMOVE;
}
int main(void) {
  gtk_init();adw_init();loop=g_main_loop_new(NULL,FALSE);
  window=GTK_WINDOW(gtk_window_new());
  page=g_object_new(CC_TYPE_NODALIX_UPDATES_PAGE,NULL);
  gtk_window_set_child(window,GTK_WIDGET(page));gtk_window_present(window);
  g_timeout_add(1500,check_page,NULL);g_timeout_add(3000,finish,NULL);
  g_main_loop_run(loop);g_main_loop_unref(loop);return 0;
}
