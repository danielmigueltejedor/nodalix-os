/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "cc-nodalix-background.h"
#include <adwaita.h>
#include <json-glib/json-glib.h>

#define BUS "com.nodalix.Settings"
#define OBJECT "/com/nodalix/Settings"
#define IFACE "com.nodalix.Settings1"
#define HANABI "io.github.jeffshee.HanabiRenderer"
#define HANABI_UUID "hanabi-extension@jeffshee.github.io"
#define WIDTH 144
#define HEIGHT 108

struct _CcNodalixBackgroundChooser {
  GtkBox parent_instance;
  GDBusProxy *proxy;
  GCancellable *cancel;
  GtkFlowBox *gallery;
  GtkLabel *status, *details;
  GtkSearchEntry *search;
  GtkDropDown *category, *resolution, *frame_rate, *pause;
  GtkCheckButton *hardware, *offload;
  GtkWidget *toolbar, *apply, *preview, *favorite, *remove, *stop, *optimize, *cancel_optimization;
  gboolean syncing_options, catalog_fetching, catalog_pending;
  char *active_path;
  guint subscriptions[3];
  gboolean connecting, fetching, refresh_pending, busy, operation_error;
};
G_DEFINE_TYPE (CcNodalixBackgroundChooser, cc_nodalix_background_chooser, GTK_TYPE_BOX)

static void fetch_state (CcNodalixBackgroundChooser *self);
static void fetch_gallery (CcNodalixBackgroundChooser *self);
static void update_selection (CcNodalixBackgroundChooser *self);
static void update_options (CcNodalixBackgroundChooser *self, JsonObject *options);

static void
show_error (CcNodalixBackgroundChooser *self, GError *error)
{
  if (!error || g_error_matches (error, G_IO_ERROR, G_IO_ERROR_CANCELLED))
    return;
  g_dbus_error_strip_remote_error (error);
  gtk_label_set_text (self->status, error->message);
  self->operation_error = TRUE;
}
static void state_ready (GObject *source, GAsyncResult *result, gpointer data)
{
  CcNodalixBackgroundChooser *self=data;
  g_autoptr(GError) error=NULL;
  g_autoptr(GVariant) value=g_dbus_proxy_call_finish (G_DBUS_PROXY(source),result,&error);
  self->fetching=FALSE;
  if (value) {
    const char *text; g_variant_get(value,"(&s)",&text);
    g_autoptr(JsonParser) parser=json_parser_new();
    if (json_parser_load_from_data(parser,text,-1,&error)) {
      JsonObject *state=json_node_get_object(json_parser_get_root(parser));
      if (!self->busy && !self->operation_error)
        gtk_label_set_text(self->status,json_object_get_string_member(state,"message"));
      const char *path=json_object_get_string_member(state,"path");
      gboolean selected=json_object_get_boolean_member(state,"requested");
      g_free (self->active_path);
      self->active_path = g_strdup (path);
      gtk_widget_set_sensitive (self->stop, selected && !self->busy);
      if (json_object_has_member (state, "options"))
        update_options (self, json_object_get_object_member (state, "options"));
      for (int i=0;;i++) {
        GtkFlowBoxChild *child=gtk_flow_box_get_child_at_index(self->gallery,i);
        if (!child) break;
        gboolean active=selected&&g_strcmp0(path,g_object_get_data(G_OBJECT(child),"path"))==0;
        if (active) gtk_widget_add_css_class(GTK_WIDGET(child),"active-item");
        else gtk_widget_remove_css_class(GTK_WIDGET(child),"active-item");
        gtk_accessible_update_state(GTK_ACCESSIBLE(child),GTK_ACCESSIBLE_STATE_CHECKED,active,-1);
      }
      update_selection (self);
    }
  } else if (!g_error_matches(error,G_IO_ERROR,G_IO_ERROR_CANCELLED)) {
    g_dbus_error_strip_remote_error(error);
    gtk_label_set_text(self->status,error->message);
  }
  if (self->refresh_pending)
    fetch_state (self);
  g_object_unref(self);
}
static void fetch_state (CcNodalixBackgroundChooser *self)
{
  self->refresh_pending = FALSE;
  if (!self->proxy || !gtk_widget_get_mapped (GTK_WIDGET (self)))
    return;
  if (self->fetching) {
    self->refresh_pending = TRUE;
    return;
  }
  self->fetching=TRUE;
  g_dbus_proxy_call(self->proxy,"GetWallpaperState",NULL,G_DBUS_CALL_FLAGS_NONE,4000,self->cancel,state_ready,g_object_ref(self));
}
static void
wallpaper_signal (GDBusConnection *connection, const gchar *sender_name,
                  const gchar *object_path, const gchar *interface_name,
                  const gchar *signal_name, GVariant *parameters, gpointer data)
{
  fetch_state (data);
}

static void
settings_signal (GDBusProxy *proxy, const gchar *sender_name, const gchar *signal_name,
                 GVariant *parameters, CcNodalixBackgroundChooser *self)
{
  if (g_str_equal (signal_name, "WallpaperChanged"))
    fetch_state (self);
  else if (g_str_equal (signal_name, "WallpaperCatalogChanged"))
    fetch_gallery (self);
}
static void
wallpaper_done (GObject *source, GAsyncResult *result, gpointer data)
{
  g_autoptr(CcNodalixBackgroundChooser) self = data;
  g_autoptr(GError) error = NULL;
  g_autoptr(GVariant) value = g_dbus_proxy_call_finish (G_DBUS_PROXY (source), result, &error);
  self->busy = FALSE;
  gtk_widget_set_visible (self->cancel_optimization, FALSE);
  gtk_widget_set_sensitive (self->toolbar, TRUE);
  show_error (self, error);
  if (value && g_variant_is_of_type (value, G_VARIANT_TYPE ("(s)"))) {
    const char *text;
    g_variant_get (value, "(&s)", &text);
    g_autoptr(JsonParser) parser = json_parser_new ();
    if (json_parser_load_from_data (parser, text, -1, NULL)) {
      JsonObject *report = json_node_get_object (json_parser_get_root (parser));
      JsonArray *errors = json_object_get_array_member (report, "errors");
      g_autoptr(GString) message = g_string_new (NULL);
      g_string_append_printf (message, "Importados: %" G_GINT64_FORMAT " · Ya existían: %" G_GINT64_FORMAT,
                              json_object_get_int_member (report, "imported"),
                              json_object_get_int_member (report, "duplicates"));
      for (guint i = 0; i < json_array_get_length (errors); i++)
        g_string_append_printf (message, "\n%s", json_array_get_string_element (errors, i));
      gtk_label_set_text (self->status, message->str);
      self->operation_error = TRUE;
    }
  }
  update_selection (self);
  fetch_state (self);
}

static void
run_operation (CcNodalixBackgroundChooser *self, const char *method, GVariant *arguments,
               const char *message)
{
  g_autoptr(GVariant) parameters = arguments ? g_variant_take_ref (arguments) : NULL;
  if (!self->proxy || self->busy)
    return;
  self->busy = TRUE;
  self->operation_error = FALSE;
  gtk_label_set_text (self->status, message);
  gtk_widget_set_sensitive (self->toolbar, FALSE);
  gtk_widget_set_sensitive (self->stop, FALSE);
  update_selection (self);
  g_dbus_proxy_call (self->proxy, method, parameters, G_DBUS_CALL_FLAGS_NONE,
                     G_MAXINT, self->cancel, wallpaper_done, g_object_ref (self));
}

static const char *
child_path (GtkFlowBoxChild *child)
{
  return child ? g_object_get_data (G_OBJECT (child), "path") : NULL;
}

static GtkFlowBoxChild *
selected_child (CcNodalixBackgroundChooser *self)
{
  g_autoptr(GList) children = gtk_flow_box_get_selected_children (self->gallery);
  return children ? children->data : NULL;
}

static void
thumbnail_activated (GtkFlowBox *box, GtkFlowBoxChild *child, CcNodalixBackgroundChooser *self)
{
  if (!g_object_get_data (G_OBJECT (child), "unreadable"))
    run_operation (self, "SetWallpaper", g_variant_new ("(s)", child_path (child)), "Activando fondo animado…");
}

static void
selection_changed (GtkFlowBox *box, CcNodalixBackgroundChooser *self)
{
  update_selection (self);
}

static void
update_selection (CcNodalixBackgroundChooser *self)
{
  GtkFlowBoxChild *child = selected_child (self);
  const char *path = child_path (child);
  gboolean ready = path && !self->busy && self->proxy;
  gboolean readable = child && !g_object_get_data (G_OBJECT (child), "unreadable");
  gtk_widget_set_sensitive (self->apply, ready && readable);
  gtk_widget_set_sensitive (self->preview, ready && readable);
  int fps = child ? GPOINTER_TO_INT (g_object_get_data (G_OBJECT (child), "fps")) : 0;
  gtk_widget_set_sensitive (self->optimize, ready && readable && fps > 0 && fps < 6000);
  gtk_widget_set_sensitive (self->favorite, ready);
  gtk_widget_set_sensitive (self->remove, ready && g_object_get_data (G_OBJECT (child), "personal")
                            && g_strcmp0 (path, self->active_path) != 0);
  gtk_button_set_label (GTK_BUTTON (self->favorite),
                        child && g_object_get_data (G_OBJECT (child), "favorite") ? "Quitar favorito" : "Marcar favorito");
  gtk_label_set_text (self->details, child ? g_object_get_data (G_OBJECT (child), "details") : "");
}

static gboolean
filter_gallery (GtkFlowBoxChild *child, gpointer data)
{
  CcNodalixBackgroundChooser *self = data;
  const char *title = g_object_get_data (G_OBJECT (child), "title");
  const char *query = gtk_editable_get_text (GTK_EDITABLE (self->search));
  g_autofree char *name = g_utf8_casefold (title, -1);
  g_autofree char *needle = g_utf8_casefold (query, -1);
  guint category = gtk_drop_down_get_selected (self->category);
  guint resolution = gtk_drop_down_get_selected (self->resolution);
  guint frame_rate = gtk_drop_down_get_selected (self->frame_rate);
  int height = GPOINTER_TO_INT (g_object_get_data (G_OBJECT (child), "height"));
  int fps = GPOINTER_TO_INT (g_object_get_data (G_OBJECT (child), "fps"));
  return strstr (name, needle) &&
         (category != 1 || g_object_get_data (G_OBJECT (child), "personal")) &&
         (category != 2 || !g_object_get_data (G_OBJECT (child), "personal")) &&
         (category != 3 || g_object_get_data (G_OBJECT (child), "favorite")) &&
         (resolution != 1 || height >= 2160) &&
         (resolution != 2 || (height > 0 && height < 2160)) &&
         (frame_rate != 1 || (fps > 0 && fps <= 3000)) &&
         (frame_rate != 2 || fps > 3000);
}

static void
filter_changed (GtkWidget *widget, CcNodalixBackgroundChooser *self)
{
  gtk_flow_box_invalidate_filter (self->gallery);
  GtkFlowBoxChild *child = selected_child (self);
  if (child && !gtk_widget_get_child_visible (GTK_WIDGET (child)))
    gtk_flow_box_unselect_all (self->gallery);
}

static void
filter_dropdown_changed (GObject *object, GParamSpec *spec, CcNodalixBackgroundChooser *self)
{
  filter_changed (GTK_WIDGET (object), self);
}

static void
update_options (CcNodalixBackgroundChooser *self, JsonObject *options)
{
  const char *key = "pause-on-maximize-or-fullscreen";
  self->syncing_options = TRUE;
  gtk_widget_set_sensitive (GTK_WIDGET (self->pause), json_object_has_member (options, key));
  gtk_drop_down_set_selected (self->pause, json_object_get_int_member_with_default (options, key, 0));
  key = "enable-va";
  gtk_widget_set_sensitive (GTK_WIDGET (self->hardware), json_object_has_member (options, key));
  gtk_check_button_set_active (self->hardware, json_object_get_int_member_with_default (options, key, 0));
  key = "enable-graphics-offload";
  gtk_widget_set_sensitive (GTK_WIDGET (self->offload), json_object_has_member (options, key));
  gtk_check_button_set_active (self->offload, json_object_get_int_member_with_default (options, key, 0));
  self->syncing_options = FALSE;
}

static void
pause_changed (GObject *object, GParamSpec *spec, CcNodalixBackgroundChooser *self)
{
  if (!self->syncing_options)
    run_operation (self, "SetWallpaperOption",
                   g_variant_new ("(si)", "pause-on-maximize-or-fullscreen", gtk_drop_down_get_selected (self->pause)),
                   "Guardando la opción de pausa…");
}

static void
performance_changed (GtkCheckButton *button, CcNodalixBackgroundChooser *self)
{
  if (!self->syncing_options)
    run_operation (self, "SetWallpaperOption",
                   g_variant_new ("(si)", g_object_get_data (G_OBJECT (button), "option"), gtk_check_button_get_active (button)),
                   "Guardando la opción de rendimiento…");
}

static void gallery_ready (GObject *source,GAsyncResult *result,gpointer data)
{
  CcNodalixBackgroundChooser *self=data;g_autoptr(GError) error=NULL;
  g_autoptr(GVariant) value=g_dbus_proxy_call_finish(G_DBUS_PROXY(source),result,&error);
  self->catalog_fetching = FALSE;
  if (value) {
    g_autofree char *selected = g_strdup (child_path (selected_child (self)));
    const char *text;g_variant_get(value,"(&s)",&text);g_autoptr(JsonParser) parser=json_parser_new();
    if (json_parser_load_from_data(parser,text,-1,&error)) {
      JsonArray *items=json_node_get_array(json_parser_get_root(parser));
      gtk_flow_box_remove_all (self->gallery);
      for(guint i=0;i<json_array_get_length(items);i++) {
        JsonObject *item=json_array_get_object_element(items,i);
        const char *preview_path = json_object_get_string_member (item, "preview");
        GtkWidget *picture = gtk_picture_new_for_filename (*preview_path ? preview_path : NULL);
        gtk_picture_set_content_fit (GTK_PICTURE (picture), GTK_CONTENT_FIT_COVER);
        gtk_picture_set_can_shrink(GTK_PICTURE(picture),FALSE);
        gtk_widget_set_size_request(picture,WIDTH,HEIGHT);
        GtkWidget *overlay=gtk_overlay_new();gtk_widget_set_overflow(overlay,GTK_OVERFLOW_HIDDEN);
        gtk_widget_add_css_class(overlay,"background-thumbnail");gtk_overlay_set_child(GTK_OVERLAY(overlay),picture);
        GtkWidget *play=gtk_image_new_from_icon_name("media-playback-start-symbolic");
        gtk_widget_set_halign(play,GTK_ALIGN_START);gtk_widget_set_valign(play,GTK_ALIGN_END);
        gtk_widget_add_css_class(play,"slideshow-icon");gtk_overlay_add_overlay(GTK_OVERLAY(overlay),play);
        GtkWidget *check=gtk_image_new_from_icon_name("background-selected-symbolic");
        gtk_widget_set_halign(check,GTK_ALIGN_END);gtk_widget_set_valign(check,GTK_ALIGN_END);
        gtk_widget_add_css_class(check,"selected-check");gtk_overlay_add_overlay(GTK_OVERLAY(overlay),check);
        GtkWidget *tile = gtk_box_new (GTK_ORIENTATION_VERTICAL, 4);
        gtk_box_append (GTK_BOX (tile), overlay);
        GtkWidget *caption = gtk_label_new (json_object_get_string_member (item, "title"));
        gtk_label_set_ellipsize (GTK_LABEL (caption), PANGO_ELLIPSIZE_END);
        gtk_label_set_max_width_chars (GTK_LABEL (caption), 19);
        gtk_box_append (GTK_BOX (tile), caption);
        GtkWidget *child=gtk_flow_box_child_new();gtk_flow_box_child_set_child(GTK_FLOW_BOX_CHILD(child),tile);
        gtk_widget_set_halign(child,GTK_ALIGN_CENTER);gtk_widget_set_valign(child,GTK_ALIGN_CENTER);
        const char *title = json_object_get_string_member (item, "title");
        const char *path = json_object_get_string_member (item, "path");
        const char *codec = json_object_get_string_member_with_default (item, "codec", "");
        int width = json_object_get_int_member_with_default (item, "width", 0);
        int height = json_object_get_int_member_with_default (item, "height", 0);
        double fps = json_object_get_double_member_with_default (item, "fps", 0);
        double duration = json_object_get_double_member_with_default (item, "duration", 0);
        gboolean unreadable = json_object_get_boolean_member_with_default (item, "unreadable", FALSE);
        g_autofree char *size = g_format_size (json_object_get_int_member_with_default (item, "size", 0));
        char *details = width
          ? g_strdup_printf ("%s · %d × %d · %.2f FPS · %s · %.1f s · %s", title, width, height, fps, codec, duration, size)
          : g_strdup_printf ("%s · %s", title, unreadable ? "Vídeo no reproducible" : size);
        gtk_widget_set_tooltip_text (child, details);
        g_object_set_data_full (G_OBJECT (child), "details", details, g_free);
        g_object_set_data_full (G_OBJECT (child), "title", g_strdup (title), g_free);
        g_object_set_data (G_OBJECT (child), "height", GINT_TO_POINTER (height));
        g_object_set_data (G_OBJECT (child), "fps", GINT_TO_POINTER ((int) (fps * 100 + 0.5)));
        g_object_set_data (G_OBJECT (child), "unreadable", GINT_TO_POINTER (unreadable));
        g_object_set_data (G_OBJECT (child), "personal", GINT_TO_POINTER (json_object_get_boolean_member_with_default (item, "personal", FALSE)));
        g_object_set_data (G_OBJECT (child), "favorite", GINT_TO_POINTER (json_object_get_boolean_member_with_default (item, "favorite", FALSE)));
        g_object_set(child,"accessible-role",GTK_ACCESSIBLE_ROLE_TOGGLE_BUTTON,NULL);
        gtk_accessible_update_property(GTK_ACCESSIBLE(child),GTK_ACCESSIBLE_PROPERTY_LABEL,json_object_get_string_member(item,"title"),-1);
        gtk_accessible_update_state(GTK_ACCESSIBLE(child),GTK_ACCESSIBLE_STATE_CHECKED,FALSE,-1);
        g_object_set_data_full(G_OBJECT(child),"path",g_strdup(json_object_get_string_member(item,"path")),g_free);
        gtk_flow_box_insert(self->gallery,child,-1);
        if (g_strcmp0 (selected, path) == 0 && gtk_widget_get_child_visible (child))
          gtk_flow_box_select_child (self->gallery, GTK_FLOW_BOX_CHILD (child));
      }
      fetch_state(self);
    }
  } else if (!g_error_matches(error,G_IO_ERROR,G_IO_ERROR_CANCELLED)) {
    g_dbus_error_strip_remote_error(error);gtk_label_set_text(self->status,error->message);
  }
  if (self->catalog_pending)
    fetch_gallery (self);
  g_object_unref(self);
}

static void
fetch_gallery (CcNodalixBackgroundChooser *self)
{
  self->catalog_pending = FALSE;
  if (!self->proxy || !gtk_widget_get_mapped (GTK_WIDGET (self)))
    return;
  if (self->catalog_fetching) {
    self->catalog_pending = TRUE;
    return;
  }
  self->catalog_fetching = TRUE;
  g_dbus_proxy_call (self->proxy, "GetWallpapers", NULL, G_DBUS_CALL_FLAGS_NONE,
                     4000, self->cancel, gallery_ready, g_object_ref (self));
}

static void proxy_ready (GObject *source,GAsyncResult *result,gpointer data)
{
  CcNodalixBackgroundChooser *self=data;g_autoptr(GError) error=NULL;
  self->connecting = FALSE;
  self->proxy=g_dbus_proxy_new_for_bus_finish(result,&error);
  if (self->proxy) {
    GDBusConnection *connection = g_dbus_proxy_get_connection (self->proxy);
    g_signal_connect_object (self->proxy, "g-signal", G_CALLBACK (settings_signal), self, 0);
    self->subscriptions[0] = g_dbus_connection_signal_subscribe (connection,
        "org.gnome.Shell", "org.gnome.Shell.Extensions", "ExtensionStateChanged",
        "/org/gnome/Shell", HANABI_UUID, G_DBUS_SIGNAL_FLAGS_NONE, wallpaper_signal, self, NULL);
    self->subscriptions[1] = g_dbus_connection_signal_subscribe (connection,
        "org.freedesktop.DBus", "org.freedesktop.DBus", "NameOwnerChanged",
        "/org/freedesktop/DBus", HANABI, G_DBUS_SIGNAL_FLAGS_NONE, wallpaper_signal, self, NULL);
    self->subscriptions[2] = g_dbus_connection_signal_subscribe (connection,
        HANABI, HANABI, "isPlayingChanged", "/io/github/jeffshee/HanabiRenderer",
        NULL, G_DBUS_SIGNAL_FLAGS_NONE, wallpaper_signal, self, NULL);
    gtk_widget_set_sensitive (self->toolbar, TRUE);
    fetch_gallery (self);
  } else if (!g_error_matches(error,G_IO_ERROR,G_IO_ERROR_CANCELLED)) gtk_label_set_text(self->status,error->message);
  g_object_unref(self);
}
static void chooser_dispose (GObject *object)
{
  CcNodalixBackgroundChooser *self=CC_NODALIX_BACKGROUND_CHOOSER(object);
  if (self->proxy) {
    GDBusConnection *connection = g_dbus_proxy_get_connection (self->proxy);
    for (guint i = 0; i < G_N_ELEMENTS (self->subscriptions); i++)
      g_dbus_connection_signal_unsubscribe (connection, self->subscriptions[i]);
  }
  if(self->cancel)g_cancellable_cancel(self->cancel);
  g_clear_object(&self->cancel);g_clear_object(&self->proxy);
  g_clear_pointer (&self->active_path, g_free);
  G_OBJECT_CLASS(cc_nodalix_background_chooser_parent_class)->dispose(object);
}
static void
chooser_map (GtkWidget *widget)
{
  CcNodalixBackgroundChooser *self = CC_NODALIX_BACKGROUND_CHOOSER (widget);
  GTK_WIDGET_CLASS (cc_nodalix_background_chooser_parent_class)->map (widget);
  if (self->proxy) {
    fetch_gallery (self);
  } else if (!self->connecting) {
    self->connecting = TRUE;
    g_dbus_proxy_new_for_bus (G_BUS_TYPE_SESSION, G_DBUS_PROXY_FLAGS_NONE, NULL,
        BUS, OBJECT, IFACE, self->cancel, proxy_ready, g_object_ref (self));
  }
}

static void cc_nodalix_background_chooser_class_init (CcNodalixBackgroundChooserClass *klass)
{
  G_OBJECT_CLASS (klass)->dispose = chooser_dispose;
  GTK_WIDGET_CLASS (klass)->map = chooser_map;
}
static void
file_dialog_done (GObject *source, GAsyncResult *result, gpointer data)
{
  g_autoptr(CcNodalixBackgroundChooser) self = data;
  g_autoptr(GError) error = NULL;
  g_autoptr(GListModel) files = gtk_file_dialog_open_multiple_finish (GTK_FILE_DIALOG (source), result, &error);
  if (!files) {
    if (!g_error_matches (error, GTK_DIALOG_ERROR, GTK_DIALOG_ERROR_DISMISSED))
      show_error (self, error);
    return;
  }

  GVariantBuilder paths;
  g_variant_builder_init (&paths, G_VARIANT_TYPE ("as"));
  for (guint i = 0; i < g_list_model_get_n_items (files); i++) {
    g_autoptr(GFile) file = g_list_model_get_item (files, i);
    g_autofree char *path = g_file_get_path (file);
    if (path)
      g_variant_builder_add (&paths, "s", path);
  }
  g_autoptr(GVariant) selection = g_variant_ref_sink (g_variant_builder_end (&paths));
  if (g_variant_n_children (selection))
    run_operation (self, "ImportWallpapers", g_variant_new ("(@as)", selection), "Importando vídeos…");
  else
    gtk_label_set_text (self->status, "Elige vídeos guardados en el equipo");
}

static void
import_clicked (GtkButton *button, CcNodalixBackgroundChooser *self)
{
  g_autoptr(GtkFileDialog) dialog = gtk_file_dialog_new ();
  g_autoptr(GtkFileFilter) filter = gtk_file_filter_new ();
  g_autoptr(GListStore) filters = g_list_store_new (GTK_TYPE_FILE_FILTER);
  gtk_file_filter_set_name (filter, "Vídeos MP4, WebM y MKV");
  gtk_file_filter_add_suffix (filter, "mp4");
  gtk_file_filter_add_suffix (filter, "webm");
  gtk_file_filter_add_suffix (filter, "mkv");
  g_list_store_append (filters, filter);
  gtk_file_dialog_set_filters (dialog, G_LIST_MODEL (filters));
  gtk_file_dialog_set_title (dialog, "Añadir fondos animados");
  const char *pictures = g_get_user_special_dir (G_USER_DIRECTORY_PICTURES);
  if (pictures) {
    g_autofree char *collection = g_build_filename (pictures, "Nodalix", "Fondos", "Animados", NULL);
    g_autoptr(GFile) folder = g_file_new_for_path (g_file_test (collection, G_FILE_TEST_IS_DIR) ? collection : pictures);
    gtk_file_dialog_set_initial_folder (dialog, folder);
  }
  gtk_file_dialog_open_multiple (dialog, GTK_WINDOW (gtk_widget_get_root (GTK_WIDGET (self))),
                                 self->cancel, file_dialog_done, g_object_ref (self));
}

static void
delete_response (AdwAlertDialog *dialog, const char *response, CcNodalixBackgroundChooser *self)
{
  if (g_str_equal (response, "delete"))
    run_operation (self, "DeleteWallpapers",
                   g_variant_ref (g_object_get_data (G_OBJECT (dialog), "delete-arguments")),
                   "Borrando fondos personales…");
}

static void
confirm_delete (CcNodalixBackgroundChooser *self, GVariant *paths, gboolean duplicates)
{
  g_autofree char *body = duplicates
    ? g_strdup_printf ("Se borrarán %zu copias idénticas de tu colección personal.",
                       g_variant_n_children (paths))
    : g_strdup ("Se borrará el vídeo de tu colección personal.");
  AdwDialog *dialog = adw_alert_dialog_new (duplicates ? "¿Borrar duplicados?" : "¿Borrar el fondo?", body);
  adw_alert_dialog_add_responses (ADW_ALERT_DIALOG (dialog), "cancel", "Cancelar", "delete", "Borrar", NULL);
  adw_alert_dialog_set_response_appearance (ADW_ALERT_DIALOG (dialog), "delete", ADW_RESPONSE_DESTRUCTIVE);
  adw_alert_dialog_set_close_response (ADW_ALERT_DIALOG (dialog), "cancel");
  adw_alert_dialog_set_default_response (ADW_ALERT_DIALOG (dialog), "cancel");
  g_object_set_data_full (G_OBJECT (dialog), "delete-arguments",
                          g_variant_ref_sink (g_variant_new ("(@asb)", paths, duplicates)),
                          (GDestroyNotify) g_variant_unref);
  g_signal_connect_object (dialog, "response", G_CALLBACK (delete_response), self, 0);
  adw_dialog_present (dialog, GTK_WIDGET (self));
}

static void
duplicates_ready (GObject *source, GAsyncResult *result, gpointer data)
{
  g_autoptr(CcNodalixBackgroundChooser) self = data;
  g_autoptr(GError) error = NULL;
  g_autoptr(GVariant) reply = g_dbus_proxy_call_finish (G_DBUS_PROXY (source), result, &error);
  self->busy = FALSE;
  self->operation_error = TRUE;
  gtk_widget_set_sensitive (self->toolbar, TRUE);
  update_selection (self);
  show_error (self, error);
  if (!reply)
    return;
  g_autoptr(GVariant) paths = g_variant_get_child_value (reply, 0);
  if (g_variant_n_children (paths)) {
    gtk_label_set_text (self->status, "Se encontraron vídeos idénticos en la colección personal");
    if (gtk_widget_get_mapped (GTK_WIDGET (self)))
      confirm_delete (self, paths, TRUE);
  } else {
    gtk_label_set_text (self->status, "No hay duplicados personales que se puedan borrar");
  }
  fetch_state (self);
}

static void
duplicates_clicked (GtkButton *button, CcNodalixBackgroundChooser *self)
{
  if (!self->proxy || self->busy)
    return;
  self->busy = TRUE;
  gtk_widget_set_sensitive (self->toolbar, FALSE);
  update_selection (self);
  gtk_label_set_text (self->status, "Comparando el contenido de los vídeos…");
  g_dbus_proxy_call (self->proxy, "FindWallpaperDuplicates", NULL, G_DBUS_CALL_FLAGS_NONE,
                     G_MAXINT, self->cancel, duplicates_ready, g_object_ref (self));
}

static void
remove_clicked (GtkButton *button, CcNodalixBackgroundChooser *self)
{
  const char *path = child_path (selected_child (self));
  if (path) {
    g_autoptr(GVariant) paths = g_variant_ref_sink (g_variant_new_strv (&path, 1));
    confirm_delete (self, paths, FALSE);
  }
}

static void
favorite_clicked (GtkButton *button, CcNodalixBackgroundChooser *self)
{
  GtkFlowBoxChild *child = selected_child (self);
  if (child)
    run_operation (self, "FavoriteWallpaper",
                   g_variant_new ("(sb)", child_path (child), g_object_get_data (G_OBJECT (child), "favorite") == NULL),
                   "Guardando favoritos…");
}

static void
apply_clicked (GtkButton *button, CcNodalixBackgroundChooser *self)
{
  GtkFlowBoxChild *child = selected_child (self);
  if (child)
    thumbnail_activated (self->gallery, child, self);
}

static void
stop_clicked (GtkButton *button, CcNodalixBackgroundChooser *self)
{
  run_operation (self, "StopWallpaper", NULL, "Deteniendo el fondo animado…");
}

static void
preview_clicked (GtkButton *button, CcNodalixBackgroundChooser *self)
{
  const char *path = child_path (selected_child (self));
  if (!path)
    return;
  g_autoptr(GFile) file = g_file_new_for_path (path);
  GtkWidget *video = gtk_video_new_for_file (file);
  GtkMediaStream *stream = gtk_video_get_media_stream (GTK_VIDEO (video));
  if (stream)
    gtk_media_stream_set_muted (stream, TRUE);
  gtk_video_set_autoplay (GTK_VIDEO (video), TRUE);
  gtk_video_set_loop (GTK_VIDEO (video), TRUE);
  gtk_widget_set_size_request (video, 400, 225);
  AdwDialog *dialog = adw_alert_dialog_new ("Vista previa del fondo", NULL);
  adw_alert_dialog_set_extra_child (ADW_ALERT_DIALOG (dialog), video);
  adw_alert_dialog_add_response (ADW_ALERT_DIALOG (dialog), "close", "Cerrar");
  adw_alert_dialog_set_close_response (ADW_ALERT_DIALOG (dialog), "close");
  adw_dialog_present (dialog, GTK_WIDGET (self));
}

static void
optimize_response (AdwAlertDialog *dialog, const char *response, CcNodalixBackgroundChooser *self)
{
  if (!g_str_equal (response, "optimize") || self->busy || !self->proxy)
    return;
  gtk_widget_set_visible (self->cancel_optimization, TRUE);
  gtk_widget_set_sensitive (self->cancel_optimization, TRUE);
  run_operation (self, "OptimizeWallpaper",
                 g_variant_new ("(s)", (const char *) g_object_get_data (G_OBJECT (dialog), "path")),
                 "Creando una copia a 60 FPS. Puedes cancelar en cualquier momento…");
}

static void
optimize_clicked (GtkButton *button, CcNodalixBackgroundChooser *self)
{
  const char *path = child_path (selected_child (self));
  if (!path)
    return;
  AdwDialog *dialog = adw_alert_dialog_new ("¿Crear una copia a 60 FPS?",
      "La interpolación conserva el vídeo original y su duración. Puede tardar mucho, usar CPU "
      "y ocupar más espacio. Los movimientos rápidos pueden mostrar artefactos.");
  adw_alert_dialog_add_responses (ADW_ALERT_DIALOG (dialog), "cancel", "Cancelar", "optimize", "Crear copia", NULL);
  adw_alert_dialog_set_close_response (ADW_ALERT_DIALOG (dialog), "cancel");
  adw_alert_dialog_set_default_response (ADW_ALERT_DIALOG (dialog), "cancel");
  g_object_set_data_full (G_OBJECT (dialog), "path", g_strdup (path), g_free);
  g_signal_connect_object (dialog, "response", G_CALLBACK (optimize_response), self, 0);
  adw_dialog_present (dialog, GTK_WIDGET (self));
}

static void
cancel_optimization_clicked (GtkButton *button, CcNodalixBackgroundChooser *self)
{
  gtk_widget_set_sensitive (GTK_WIDGET (button), FALSE);
  g_dbus_proxy_call (self->proxy, "CancelWallpaperOptimization", NULL, G_DBUS_CALL_FLAGS_NONE,
                     4000, self->cancel, NULL, NULL);
}

static GtkWidget *
append_button (AdwWrapBox *row, const char *label, GCallback callback, CcNodalixBackgroundChooser *self)
{
  GtkWidget *button = gtk_button_new_with_label (label);
  adw_wrap_box_append (row, button);
  g_signal_connect_object (button, "clicked", callback, self, 0);
  return button;
}

static GtkDropDown *
append_choices (AdwWrapBox *row, const char *const *choices, const char *label,
                GCallback callback, CcNodalixBackgroundChooser *self)
{
  GtkDropDown *dropdown = GTK_DROP_DOWN (gtk_drop_down_new_from_strings (choices));
  gtk_accessible_update_property (GTK_ACCESSIBLE (dropdown), GTK_ACCESSIBLE_PROPERTY_LABEL, label, -1);
  adw_wrap_box_append (row, GTK_WIDGET (dropdown));
  g_signal_connect_object (dropdown, "notify::selected", callback, self, 0);
  return dropdown;
}

static GtkCheckButton *
append_performance (AdwWrapBox *row, const char *label, const char *key, CcNodalixBackgroundChooser *self)
{
  GtkCheckButton *button = GTK_CHECK_BUTTON (gtk_check_button_new_with_label (label));
  g_object_set_data (G_OBJECT (button), "option", (gpointer) key);
  adw_wrap_box_append (row, GTK_WIDGET (button));
  gtk_widget_set_sensitive (GTK_WIDGET (button), FALSE);
  g_signal_connect_object (button, "toggled", G_CALLBACK (performance_changed), self, 0);
  return button;
}

static AdwWrapBox *
append_row (CcNodalixBackgroundChooser *self)
{
  GtkWidget *row = adw_wrap_box_new ();
  adw_wrap_box_set_child_spacing (ADW_WRAP_BOX (row), 6);
  adw_wrap_box_set_line_spacing (ADW_WRAP_BOX (row), 6);
  gtk_box_append (GTK_BOX (self), row);
  return ADW_WRAP_BOX (row);
}

static void
cc_nodalix_background_chooser_init (CcNodalixBackgroundChooser *self)
{
  const char *const categories[] = {"Todos", "Personales", "Nodalix", "Favoritos", NULL};
  const char *const resolutions[] = {"Cualquier resolución", "4K y superior", "Menos de 4K", NULL};
  const char *const rates[] = {"Cualquier FPS", "Hasta 30 FPS", "Más de 30 FPS", NULL};
  const char *const pause_options[] = {"Nunca pausar", "Pausar si se cubre un monitor", "Pausar si se cubren todos", NULL};

  self->cancel = g_cancellable_new ();
  gtk_orientable_set_orientation (GTK_ORIENTABLE (self), GTK_ORIENTATION_VERTICAL);
  gtk_box_set_spacing (GTK_BOX (self), 9);
  gtk_widget_set_margin_start (GTK_WIDGET (self), 12);
  gtk_widget_set_margin_end (GTK_WIDGET (self), 12);
  gtk_widget_set_margin_top (GTK_WIDGET (self), 12);
  gtk_widget_set_margin_bottom (GTK_WIDGET (self), 12);

  AdwWrapBox *toolbar = append_row (self);
  self->toolbar = GTK_WIDGET (toolbar);
  append_button (toolbar, "Añadir fondo animado", G_CALLBACK (import_clicked), self);
  append_button (toolbar, "Buscar duplicados", G_CALLBACK (duplicates_clicked), self);
  self->stop = append_button (toolbar, "Detener", G_CALLBACK (stop_clicked), self);
  gtk_widget_set_sensitive (self->toolbar, FALSE);

  self->search = GTK_SEARCH_ENTRY (gtk_search_entry_new ());
  gtk_search_entry_set_placeholder_text (self->search, "Buscar fondos animados");
  gtk_box_append (GTK_BOX (self), GTK_WIDGET (self->search));
  g_signal_connect_object (self->search, "search-changed", G_CALLBACK (filter_changed), self, 0);
  AdwWrapBox *filters = append_row (self);
  self->category = append_choices (filters, categories, "Categoría", G_CALLBACK (filter_dropdown_changed), self);
  self->resolution = append_choices (filters, resolutions, "Resolución", G_CALLBACK (filter_dropdown_changed), self);
  self->frame_rate = append_choices (filters, rates, "Frecuencia de fotogramas", G_CALLBACK (filter_dropdown_changed), self);

  self->gallery = GTK_FLOW_BOX (gtk_flow_box_new ());
  gtk_widget_add_css_class (GTK_WIDGET (self->gallery), "background-flowbox");
  gtk_flow_box_set_homogeneous (self->gallery, TRUE);
  gtk_flow_box_set_selection_mode (self->gallery, GTK_SELECTION_SINGLE);
  gtk_flow_box_set_max_children_per_line (self->gallery, 8);
  gtk_flow_box_set_row_spacing (self->gallery, 12);
  gtk_flow_box_set_column_spacing (self->gallery, 12);
  gtk_flow_box_set_activate_on_single_click (self->gallery, FALSE);
  gtk_flow_box_set_filter_func (self->gallery, filter_gallery, self, NULL);
  gtk_box_append (GTK_BOX (self), GTK_WIDGET (self->gallery));

  self->details = GTK_LABEL (gtk_label_new (""));
  gtk_label_set_wrap (self->details, TRUE);
  gtk_label_set_xalign (self->details, 0);
  gtk_box_append (GTK_BOX (self), GTK_WIDGET (self->details));
  AdwWrapBox *actions = append_row (self);
  self->apply = append_button (actions, "Usar como fondo", G_CALLBACK (apply_clicked), self);
  self->preview = append_button (actions, "Vista previa", G_CALLBACK (preview_clicked), self);
  self->favorite = append_button (actions, "Marcar favorito", G_CALLBACK (favorite_clicked), self);
  self->remove = append_button (actions, "Borrar", G_CALLBACK (remove_clicked), self);
  gtk_widget_add_css_class (self->remove, "destructive-action");

  AdwWrapBox *playback = append_row (self);
  self->optimize = append_button (playback, "Optimizar fluidez…", G_CALLBACK (optimize_clicked), self);
  self->cancel_optimization = append_button (playback, "Cancelar interpolación", G_CALLBACK (cancel_optimization_clicked), self);
  gtk_widget_set_visible (self->cancel_optimization, FALSE);
  self->pause = append_choices (playback, pause_options, "Pausa automática", G_CALLBACK (pause_changed), self);
  gtk_widget_set_sensitive (GTK_WIDGET (self->pause), FALSE);
  AdwWrapBox *performance = append_row (self);
  self->hardware = append_performance (performance, "Decodificación VA-API", "enable-va", self);
  self->offload = append_performance (performance, "Descarga gráfica", "enable-graphics-offload", self);
  self->status = GTK_LABEL (gtk_label_new ("Cargando fondos animados…"));
  gtk_label_set_wrap (self->status, TRUE);
  gtk_label_set_xalign (self->status, 0);
  gtk_widget_add_css_class (GTK_WIDGET (self->status), "dim-label");
  gtk_box_append (GTK_BOX (self), GTK_WIDGET (self->status));
  g_signal_connect_object (self->gallery, "child-activated", G_CALLBACK (thumbnail_activated), self, 0);
  g_signal_connect_object (self->gallery, "selected-children-changed", G_CALLBACK (selection_changed), self, 0);
  update_selection (self);
}
