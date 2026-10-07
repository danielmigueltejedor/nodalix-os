/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "cc-nodalix-pages.h"
#include <json-glib/json-glib.h>

#define BUS "com.nodalix.Settings"
#define OBJECT "/com/nodalix/Settings"
#define IFACE "com.nodalix.Settings1"

struct _CcNodalixUpdatesPage {
  AdwNavigationPage parent_instance;
  GDBusProxy *proxy;
  GCancellable *cancel;
  GtkLabel *status;
  GtkProgressBar *progress;
  GtkTextBuffer *log;
  AdwActionRow *rows[5];
  GtkWidget *buttons[7];
  guint timer;
  gboolean fetching;
};
G_DEFINE_TYPE(CcNodalixUpdatesPage, cc_nodalix_updates_page, ADW_TYPE_NAVIGATION_PAGE)
static const char *kinds[] = {"nodalix", "system", "apps", "flatpak", "firmware"};
static const char *titles[] = {"Nodalix", "Programas y kernel", "Aplicaciones Nodalix", "Aplicaciones Flatpak", "Firmware"};

static void fetch_updates(CcNodalixUpdatesPage *self);
static void state_ready(GObject *source, GAsyncResult *result, gpointer data)
{
  CcNodalixUpdatesPage *self = data;
  g_autoptr(GError) error = NULL;
  g_autoptr(GVariant) value = g_dbus_proxy_call_finish(G_DBUS_PROXY(source), result, &error);
  self->fetching = FALSE;
  if (value) {
    const char *text;
    g_variant_get(value, "(&s)", &text);
    g_autoptr(JsonParser) parser = json_parser_new();
    if (json_parser_load_from_data(parser, text, -1, &error)) {
      JsonObject *state = json_node_get_object(json_parser_get_root(parser));
      gboolean busy = json_object_get_boolean_member(state, "busy") || json_object_get_boolean_member(state, "checking");
      const char *message = json_object_get_string_member(state, "message");
      gtk_label_set_text(self->status, message);
      gtk_widget_set_visible(GTK_WIDGET(self->progress), busy);
      if (busy) gtk_progress_bar_pulse(self->progress);
      gtk_text_buffer_set_text(self->log, json_object_get_string_member(state, "log"), -1);
      JsonArray *rows = json_object_get_array_member(state, "sources");
      for (guint i = 0; i < MIN(5, json_array_get_length(rows)); i++) {
        JsonObject *row = json_array_get_object_element(rows, i);
        adw_action_row_set_subtitle(self->rows[i], json_object_get_string_member(row, "detail"));
        gtk_widget_set_sensitive(self->buttons[i], !busy && json_object_get_boolean_member(row, "available"));
      }
      gtk_widget_set_sensitive(self->buttons[5], !busy);
      gtk_widget_set_sensitive(self->buttons[6], !busy);
      if (json_object_get_boolean_member(state, "reboot_required") && !busy) {
        g_autofree char *with_reboot = g_strconcat(message, " · Reinicia para cargar los cambios del kernel y los servicios.", NULL);
        gtk_label_set_text(self->status, with_reboot);
      }
    }
  } else if (!g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
    gtk_label_set_text(self->status, error->message);
  }
  g_object_unref(self);
}
static void fetch_updates(CcNodalixUpdatesPage *self)
{
  if (!self->proxy || self->fetching) return;
  self->fetching = TRUE;
  g_dbus_proxy_call(self->proxy, "GetUpdates", NULL, G_DBUS_CALL_FLAGS_NONE, 2500,
                    self->cancel, state_ready, g_object_ref(self));
}
static gboolean poll_updates(gpointer data) { fetch_updates(data); return G_SOURCE_CONTINUE; }
static void operation_ready(GObject *source, GAsyncResult *result, gpointer data)
{
  CcNodalixUpdatesPage *self = data;
  g_autoptr(GError) error = NULL;
  g_autoptr(GVariant) value = g_dbus_proxy_call_finish(G_DBUS_PROXY(source), result, &error);
  if (!value && !g_error_matches(error,G_IO_ERROR,G_IO_ERROR_CANCELLED)) gtk_label_set_text(self->status,error->message);
  else fetch_updates(self);
  g_object_unref(self);
}
static void update_clicked(GtkButton *button, CcNodalixUpdatesPage *self)
{
  const char *kind = g_object_get_data(G_OBJECT(button), "kind");
  if (!self->proxy) return;
  for (guint i=0;i<7;i++) gtk_widget_set_sensitive(self->buttons[i],FALSE);
  g_dbus_proxy_call(self->proxy, kind ? "StartUpdate" : "CheckUpdates", kind ? g_variant_new("(s)",kind) : NULL,
                    G_DBUS_CALL_FLAGS_NONE,2500,self->cancel,operation_ready,g_object_ref(self));
}
static void service_changed(GDBusProxy *proxy, const char *sender, const char *signal, GVariant *params, CcNodalixUpdatesPage *self)
{ if (g_str_equal(signal,"Changed")) fetch_updates(self); }
static void proxy_ready(GObject *source,GAsyncResult *result,gpointer data)
{
  CcNodalixUpdatesPage *self=data;
  g_autoptr(GError) error=NULL;
  self->proxy=g_dbus_proxy_new_for_bus_finish(result,&error);
  if (self->proxy) {
    g_signal_connect_object(self->proxy,"g-signal",G_CALLBACK(service_changed),self,0);
    fetch_updates(self);
  } else if (!g_error_matches(error,G_IO_ERROR,G_IO_ERROR_CANCELLED)) gtk_label_set_text(self->status,error->message);
  g_object_unref(self);
}
static void updates_dispose(GObject *object)
{
  CcNodalixUpdatesPage *self=CC_NODALIX_UPDATES_PAGE(object);
  if (self->timer) {g_source_remove(self->timer);self->timer=0;}
  if (self->cancel) g_cancellable_cancel(self->cancel);
  g_clear_object(&self->cancel);g_clear_object(&self->proxy);
  G_OBJECT_CLASS(cc_nodalix_updates_page_parent_class)->dispose(object);
}
static void cc_nodalix_updates_page_class_init(CcNodalixUpdatesPageClass *klass)
{G_OBJECT_CLASS(klass)->dispose=updates_dispose;}
static void cc_nodalix_updates_page_init(CcNodalixUpdatesPage *self)
{
  self->cancel=g_cancellable_new();
  adw_navigation_page_set_title(ADW_NAVIGATION_PAGE(self),"Actualizaciones");
  GtkWidget *toolbar=adw_toolbar_view_new();
  adw_toolbar_view_add_top_bar(ADW_TOOLBAR_VIEW(toolbar),adw_header_bar_new());
  GtkWidget *page=adw_preferences_page_new();
  AdwPreferencesGroup *group=ADW_PREFERENCES_GROUP(adw_preferences_group_new());
  adw_preferences_group_set_title(group,"Actualizaciones de Nodalix");
  adw_preferences_group_set_description(group,"Mantén al día el escritorio, las aplicaciones, los programas y el kernel.");
  for (guint i=0;i<5;i++) {
    self->rows[i]=ADW_ACTION_ROW(adw_action_row_new());
    adw_preferences_row_set_title(ADW_PREFERENCES_ROW(self->rows[i]),titles[i]);
    self->buttons[i]=gtk_button_new_with_label("Actualizar");
    gtk_widget_set_valign(self->buttons[i],GTK_ALIGN_CENTER);
    gtk_widget_set_sensitive(self->buttons[i],FALSE);
    g_object_set_data(G_OBJECT(self->buttons[i]),"kind",(gpointer)kinds[i]);
    g_signal_connect(self->buttons[i],"clicked",G_CALLBACK(update_clicked),self);
    adw_action_row_add_suffix(self->rows[i],self->buttons[i]);
    adw_preferences_group_add(group,GTK_WIDGET(self->rows[i]));
  }
  adw_preferences_page_add(ADW_PREFERENCES_PAGE(page),group);
  AdwPreferencesGroup *actions=ADW_PREFERENCES_GROUP(adw_preferences_group_new());
  GtkWidget *box=gtk_box_new(GTK_ORIENTATION_VERTICAL,12);
  GtkWidget *buttons=gtk_box_new(GTK_ORIENTATION_HORIZONTAL,12);
  gtk_widget_set_halign(buttons,GTK_ALIGN_CENTER);
  self->buttons[5]=gtk_button_new_with_label("Buscar actualizaciones");
  self->buttons[6]=gtk_button_new_with_label("Actualizar todo");
  gtk_widget_add_css_class(self->buttons[6],"suggested-action");
  g_object_set_data(G_OBJECT(self->buttons[6]),"kind","all");
  for(guint i=5;i<7;i++) {gtk_widget_set_sensitive(self->buttons[i],FALSE);g_signal_connect(self->buttons[i],"clicked",G_CALLBACK(update_clicked),self);gtk_box_append(GTK_BOX(buttons),self->buttons[i]);}
  self->status=GTK_LABEL(gtk_label_new("Conectando con el servicio de actualizaciones…"));
  gtk_label_set_wrap(self->status,TRUE);
  self->progress=GTK_PROGRESS_BAR(gtk_progress_bar_new());
  gtk_widget_set_visible(GTK_WIDGET(self->progress),FALSE);
  gtk_box_append(GTK_BOX(box),buttons);gtk_box_append(GTK_BOX(box),GTK_WIDGET(self->status));gtk_box_append(GTK_BOX(box),GTK_WIDGET(self->progress));
  adw_preferences_group_add(actions,box);adw_preferences_page_add(ADW_PREFERENCES_PAGE(page),actions);
  AdwPreferencesGroup *details=ADW_PREFERENCES_GROUP(adw_preferences_group_new());
  GtkWidget *expander=adw_expander_row_new();
  adw_preferences_row_set_title(ADW_PREFERENCES_ROW(expander),"Detalles de la operación");
  GtkWidget *scroller=gtk_scrolled_window_new();
  gtk_widget_set_size_request(scroller,-1,260);
  GtkWidget *text=gtk_text_view_new();gtk_text_view_set_editable(GTK_TEXT_VIEW(text),FALSE);gtk_text_view_set_monospace(GTK_TEXT_VIEW(text),TRUE);gtk_text_view_set_wrap_mode(GTK_TEXT_VIEW(text),GTK_WRAP_WORD_CHAR);
  self->log=gtk_text_view_get_buffer(GTK_TEXT_VIEW(text));
  gtk_scrolled_window_set_child(GTK_SCROLLED_WINDOW(scroller),text);
  adw_expander_row_add_row(ADW_EXPANDER_ROW(expander),scroller);adw_preferences_group_add(details,expander);adw_preferences_page_add(ADW_PREFERENCES_PAGE(page),details);
  adw_toolbar_view_set_content(ADW_TOOLBAR_VIEW(toolbar),page);adw_navigation_page_set_child(ADW_NAVIGATION_PAGE(self),toolbar);
  self->timer=g_timeout_add_seconds(3,poll_updates,self);
  g_dbus_proxy_new_for_bus(G_BUS_TYPE_SESSION,G_DBUS_PROXY_FLAGS_NONE,NULL,BUS,OBJECT,IFACE,self->cancel,proxy_ready,g_object_ref(self));
}
