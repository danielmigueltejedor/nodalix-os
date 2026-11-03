/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "cc-nodalix-storage.h"
#include <json-glib/json-glib.h>

#ifndef NODALIX_STORAGE_HELPER
#define NODALIX_STORAGE_HELPER "/usr/lib/nodalix/storage-manager"
#endif

struct _CcNodalixStoragePage {
  AdwNavigationPage parent_instance;
  AdwComboRow *category;
  GtkProgressBar *usage;
  GtkLabel *capacity;
  GtkLabel *status;
  GtkListBox *items;
  GtkWidget *refresh;
  GCancellable *cancel;
  JsonParser *data;
  gboolean busy;
};

G_DEFINE_TYPE (CcNodalixStoragePage, cc_nodalix_storage_page, ADW_TYPE_NAVIGATION_PAGE)

static const char *keys[] = { "apps", "documents", "music", "media", "downloads", "development" };
static const char *titles[] = { "Aplicaciones", "Documentos grandes", "Música", "Fotos y vídeo", "Descargas", "Herramientas de desarrollo", NULL };
static const char *icons[] = { "application-x-executable", "text-x-generic", "audio-x-generic", "image-x-generic", "folder-download", "applications-development" };

static void scan (CcNodalixStoragePage *self);

static void
set_busy (CcNodalixStoragePage *self, gboolean busy)
{
  self->busy = busy;
  gtk_widget_set_sensitive (self->refresh, !busy);
  gtk_widget_set_sensitive (GTK_WIDGET (self->items), !busy);
}

static void
trash_finished (GObject *source, GAsyncResult *result, gpointer data)
{
  g_autoptr(CcNodalixStoragePage) self = data;
  g_autoptr(GError) error = NULL;
  g_autofree char *stderr_text = NULL;
  gboolean ok = g_subprocess_communicate_utf8_finish (G_SUBPROCESS (source), result, NULL, &stderr_text, &error);
  set_busy (self, FALSE);
  if (ok && g_subprocess_get_successful (G_SUBPROCESS (source))) {
    scan (self);
  } else if (!g_error_matches (error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
    gtk_label_set_text (self->status, error ? error->message : stderr_text && *stderr_text ? stderr_text : "No se pudo enviar a la papelera.");
  }
}

static void
confirm_response (AdwAlertDialog *dialog, const char *response, CcNodalixStoragePage *self)
{
  g_autoptr(GError) error = NULL;
  g_autoptr(GSubprocess) process = NULL;
  const char *action = g_object_get_data (G_OBJECT (dialog), "action");
  const char *target = g_object_get_data (G_OBJECT (dialog), "target");
  if (!g_str_equal (response, "trash") || self->busy)
    return;
  process = g_subprocess_new (G_SUBPROCESS_FLAGS_STDOUT_PIPE | G_SUBPROCESS_FLAGS_STDERR_PIPE,
                              &error, NODALIX_STORAGE_HELPER, action, target, NULL);
  if (!process) {
    gtk_label_set_text (self->status, error->message);
    return;
  }
  set_busy (self, TRUE);
  gtk_label_set_text (self->status, "Enviando a la papelera…");
  g_subprocess_communicate_utf8_async (process, NULL, self->cancel, trash_finished, g_object_ref (self));
}

static void
trash_clicked (GtkButton *button, CcNodalixStoragePage *self)
{
  const char *target = g_object_get_data (G_OBJECT (button), "target");
  const char *name = g_object_get_data (G_OBJECT (button), "name");
  gboolean app = GPOINTER_TO_INT (g_object_get_data (G_OBJECT (button), "app"));
  g_autofree char *heading = g_strdup_printf (app ? "¿Desinstalar %s?" : "¿Enviar %s a la papelera?", name);
  AdwDialog *dialog = adw_alert_dialog_new (heading, app ? "Tus documentos y ajustes se conservan. Si es una app del sistema, puede pedir autenticación." : "Puedes recuperar el elemento desde la papelera mientras no se vacíe.");
  adw_alert_dialog_add_responses (ADW_ALERT_DIALOG (dialog), "cancel", "Cancelar", "trash", app ? "Desinstalar" : "Enviar a la papelera", NULL);
  adw_alert_dialog_set_close_response (ADW_ALERT_DIALOG (dialog), "cancel");
  adw_alert_dialog_set_default_response (ADW_ALERT_DIALOG (dialog), "cancel");
  adw_alert_dialog_set_response_appearance (ADW_ALERT_DIALOG (dialog), "trash", ADW_RESPONSE_DESTRUCTIVE);
  g_object_set_data (G_OBJECT (dialog), "action", (gpointer) (app ? "trash-app" : "trash-file"));
  g_object_set_data_full (G_OBJECT (dialog), "target", g_strdup (target), g_free);
  g_signal_connect_object (dialog, "response", G_CALLBACK (confirm_response), self, 0);
  adw_dialog_present (dialog, GTK_WIDGET (self));
}

static void
show_in_files (GtkButton *button, CcNodalixStoragePage *self)
{
  const char *path = g_object_get_data (G_OBJECT (button), "path");
  g_autoptr(GFile) file = g_file_new_for_path (path);
  g_autoptr(GFile) parent = g_file_get_parent (file);
  g_autofree char *uri = g_file_get_uri (parent);
  g_app_info_launch_default_for_uri_async (uri, NULL, self->cancel, NULL, NULL);
}

static void
render (CcNodalixStoragePage *self)
{
  GtkWidget *child;
  guint selected = adw_combo_row_get_selected (self->category);
  while ((child = gtk_widget_get_first_child (GTK_WIDGET (self->items))))
    gtk_list_box_remove (self->items, child);
  if (!self->data || selected >= G_N_ELEMENTS (keys))
    return;
  JsonObject *root = json_node_get_object (json_parser_get_root (self->data));
  JsonObject *categories = json_object_get_object_member (root, "categories");
  JsonArray *entries = json_object_get_array_member (categories, keys[selected]);
  gboolean app = selected == 0;
  for (guint i = 0; i < json_array_get_length (entries); i++) {
    JsonObject *item = json_array_get_object_element (entries, i);
    const char *name = json_object_get_string_member (item, "name");
    const char *detail = json_object_get_string_member (item, "detail");
    const char *target = json_object_get_string_member (item, app ? "id" : "path");
    JsonNode *size_node = json_object_get_member (item, "size");
    g_autofree char *size = JSON_NODE_HOLDS_NULL (size_node) ? g_strdup ("Tamaño no disponible") : g_format_size (json_node_get_int (size_node));
    g_autofree char *subtitle = g_strconcat (size, " · ", detail, NULL);
    g_autofree char *title_markup = g_markup_escape_text (name, -1);
    g_autofree char *subtitle_markup = g_markup_escape_text (subtitle, -1);
    AdwActionRow *row = ADW_ACTION_ROW (adw_action_row_new ());
    adw_preferences_row_set_title (ADW_PREFERENCES_ROW (row), title_markup);
    adw_action_row_set_subtitle (row, subtitle_markup);
    GtkWidget *icon = gtk_image_new_from_icon_name (icons[selected]);
    if (app) {
      const char *icon_string = json_object_get_string_member (item, "icon");
      g_autoptr(GIcon) app_icon = icon_string && *icon_string ? g_icon_new_for_string (icon_string, NULL) : NULL;
      if (app_icon)
        gtk_image_set_from_gicon (GTK_IMAGE (icon), app_icon);
    }
    gtk_image_set_pixel_size (GTK_IMAGE (icon), 32);
    adw_action_row_add_prefix (row, icon);
    if (!app) {
      GtkWidget *show = gtk_button_new_from_icon_name ("folder-open-symbolic");
      gtk_widget_set_tooltip_text (show, "Mostrar en Archivos");
      gtk_widget_set_valign (show, GTK_ALIGN_CENTER);
      g_object_set_data_full (G_OBJECT (show), "path", g_strdup (target), g_free);
      g_signal_connect_object (show, "clicked", G_CALLBACK (show_in_files), self, 0);
      adw_action_row_add_suffix (row, show);
    }
    GtkWidget *trash = gtk_button_new_from_icon_name ("user-trash-symbolic");
    gtk_widget_set_tooltip_text (trash, app ? "Desinstalar aplicación" : "Enviar a la papelera");
    gtk_widget_set_valign (trash, GTK_ALIGN_CENTER);
    g_object_set_data_full (G_OBJECT (trash), "target", g_strdup (target), g_free);
    g_object_set_data_full (G_OBJECT (trash), "name", g_strdup (name), g_free);
    g_object_set_data (G_OBJECT (trash), "app", GINT_TO_POINTER (app));
    g_signal_connect_object (trash, "clicked", G_CALLBACK (trash_clicked), self, 0);
    adw_action_row_add_suffix (row, trash);
    gtk_list_box_append (self->items, GTK_WIDGET (row));
  }
  GtkWidget *empty = gtk_label_new (selected == 1 ? "No hay documentos de 16 MB o más en las carpetas revisadas." : "No hay elementos en esta categoría.");
  gtk_label_set_wrap (GTK_LABEL (empty), TRUE);
  gtk_list_box_set_placeholder (self->items, empty);
}

static void
category_changed (GObject *row, GParamSpec *pspec, CcNodalixStoragePage *self)
{
  render (self);
}

static void
scan_finished (GObject *source, GAsyncResult *result, gpointer data)
{
  g_autoptr(CcNodalixStoragePage) self = data;
  g_autoptr(GError) error = NULL;
  g_autofree char *output = NULL;
  g_autofree char *stderr_text = NULL;
  g_autoptr(JsonParser) parser = json_parser_new ();
  set_busy (self, FALSE);
  if (!g_subprocess_communicate_utf8_finish (G_SUBPROCESS (source), result, &output, &stderr_text, &error) ||
      !g_subprocess_get_successful (G_SUBPROCESS (source)) ||
      !json_parser_load_from_data (parser, output, -1, &error)) {
    if (!g_error_matches (error, G_IO_ERROR, G_IO_ERROR_CANCELLED))
      gtk_label_set_text (self->status, error ? error->message : stderr_text && *stderr_text ? stderr_text : "No se pudo calcular el almacenamiento.");
    return;
  }
  g_set_object (&self->data, parser);
  JsonObject *root = json_node_get_object (json_parser_get_root (parser));
  JsonObject *volume = json_object_get_object_member (root, "volume");
  JsonObject *totals = json_object_get_object_member (root, "totals");
  gint64 total = json_object_get_int_member (volume, "total");
  gint64 free = json_object_get_int_member (volume, "free");
  gint64 used = json_object_get_int_member (volume, "used");
  g_autofree char *total_text = g_format_size (total);
  g_autofree char *free_text = g_format_size (free);
  g_autofree char *capacity = g_strdup_printf ("%s disponibles de %s", free_text, total_text);
  gtk_label_set_text (self->capacity, capacity);
  gtk_progress_bar_set_fraction (self->usage, total > 0 ? CLAMP ((double) used / total, 0.0, 1.0) : 0);
  guint previous = adw_combo_row_get_selected (self->category);
  g_autoptr(GtkStringList) model = gtk_string_list_new (NULL);
  for (guint i = 0; i < G_N_ELEMENTS (keys); i++) {
    g_autofree char *size = g_format_size (json_object_get_int_member (totals, keys[i]));
    g_autofree char *label = g_strdup_printf ("%s · %s", titles[i], size);
    gtk_string_list_append (model, label);
  }
  adw_combo_row_set_model (self->category, G_LIST_MODEL (model));
  adw_combo_row_set_selected (self->category, MIN (previous, G_N_ELEMENTS (keys)-1));
  gtk_label_set_text (self->status, json_object_get_boolean_member (root, "partial") ? "Revisión parcial: se alcanzó el límite de archivos. Se muestran los elementos encontrados." : "Ordenado por tamaño. El espacio liberado puede variar si se comparten archivos.");
  render (self);
}

static void
scan (CcNodalixStoragePage *self)
{
  g_autoptr(GError) error = NULL;
  g_autoptr(GSubprocess) process = NULL;
  if (self->busy)
    return;
  process = g_subprocess_new (G_SUBPROCESS_FLAGS_STDOUT_PIPE | G_SUBPROCESS_FLAGS_STDERR_PIPE,
                              &error, NODALIX_STORAGE_HELPER, "scan", NULL);
  if (!process) {
    gtk_label_set_text (self->status, error->message);
    return;
  }
  set_busy (self, TRUE);
  gtk_label_set_text (self->status, "Calculando el almacenamiento…");
  g_subprocess_communicate_utf8_async (process, NULL, self->cancel, scan_finished, g_object_ref (self));
}

static void
shown (CcNodalixStoragePage *self)
{
  if (!self->data)
    scan (self);
}

static void
cc_nodalix_storage_page_dispose (GObject *object)
{
  CcNodalixStoragePage *self = CC_NODALIX_STORAGE_PAGE (object);
  if (self->cancel)
    g_cancellable_cancel (self->cancel);
  g_clear_object (&self->cancel);
  g_clear_object (&self->data);
  G_OBJECT_CLASS (cc_nodalix_storage_page_parent_class)->dispose (object);
}

static void
cc_nodalix_storage_page_class_init (CcNodalixStoragePageClass *klass)
{
  G_OBJECT_CLASS (klass)->dispose = cc_nodalix_storage_page_dispose;
}

static void
cc_nodalix_storage_page_init (CcNodalixStoragePage *self)
{
  self->cancel = g_cancellable_new ();
  adw_navigation_page_set_title (ADW_NAVIGATION_PAGE (self), "Almacenamiento");
  GtkWidget *toolbar = adw_toolbar_view_new ();
  GtkWidget *header = adw_header_bar_new ();
  self->refresh = gtk_button_new_from_icon_name ("view-refresh-symbolic");
  gtk_widget_set_tooltip_text (self->refresh, "Volver a calcular");
  adw_header_bar_pack_end (ADW_HEADER_BAR (header), self->refresh);
  g_signal_connect_swapped (self->refresh, "clicked", G_CALLBACK (scan), self);
  adw_toolbar_view_add_top_bar (ADW_TOOLBAR_VIEW (toolbar), header);
  GtkWidget *page = adw_preferences_page_new ();
  AdwPreferencesGroup *capacity = ADW_PREFERENCES_GROUP (adw_preferences_group_new ());
  adw_preferences_group_set_title (capacity, "Almacenamiento de este equipo");
  self->capacity = GTK_LABEL (gtk_label_new (""));
  gtk_widget_set_halign (GTK_WIDGET (self->capacity), GTK_ALIGN_START);
  self->usage = GTK_PROGRESS_BAR (gtk_progress_bar_new ());
  adw_preferences_group_add (capacity, GTK_WIDGET (self->capacity));
  adw_preferences_group_add (capacity, GTK_WIDGET (self->usage));
  adw_preferences_page_add (ADW_PREFERENCES_PAGE (page), capacity);
  AdwPreferencesGroup *manage = ADW_PREFERENCES_GROUP (adw_preferences_group_new ());
  self->category = ADW_COMBO_ROW (adw_combo_row_new ());
  adw_preferences_row_set_title (ADW_PREFERENCES_ROW (self->category), "Categoría");
  g_autoptr(GtkStringList) categories = gtk_string_list_new (titles);
  adw_combo_row_set_model (self->category, G_LIST_MODEL (categories));
  adw_preferences_group_add (manage, GTK_WIDGET (self->category));
  self->status = GTK_LABEL (gtk_label_new (""));
  gtk_label_set_wrap (self->status, TRUE);
  gtk_widget_add_css_class (GTK_WIDGET (self->status), "dim-label");
  adw_preferences_group_add (manage, GTK_WIDGET (self->status));
  self->items = GTK_LIST_BOX (gtk_list_box_new ());
  gtk_list_box_set_selection_mode (self->items, GTK_SELECTION_NONE);
  gtk_widget_add_css_class (GTK_WIDGET (self->items), "boxed-list");
  adw_preferences_group_add (manage, GTK_WIDGET (self->items));
  adw_preferences_page_add (ADW_PREFERENCES_PAGE (page), manage);
  adw_toolbar_view_set_content (ADW_TOOLBAR_VIEW (toolbar), page);
  adw_navigation_page_set_child (ADW_NAVIGATION_PAGE (self), toolbar);
  g_signal_connect (self->category, "notify::selected", G_CALLBACK (category_changed), self);
  g_signal_connect (self, "shown", G_CALLBACK (shown), NULL);
}
