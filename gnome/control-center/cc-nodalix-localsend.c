/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "cc-nodalix-pages.h"
#include <json-glib/json-glib.h>
#define BUS "com.nodalix.Settings"
#define OBJECT "/com/nodalix/Settings"
#define IFACE "com.nodalix.Settings1"
struct _CcNodalixLocalSendPage {
  AdwNavigationPage parent_instance;
  GDBusProxy *proxy;
  GCancellable *cancel;
  AdwSwitchRow *enabled, *options[3];
  AdwEntryRow *alias;
  AdwActionRow *folder;
  GtkSpinButton *minutes;
  GtkListBox *devices;
  GtkLabel *status;
  GtkWidget *prefs, *search;
  guint timer;
  gboolean fetching, loading, alias_dirty;
};
G_DEFINE_TYPE(CcNodalixLocalSendPage,cc_nodalix_localsend_page,ADW_TYPE_NAVIGATION_PAGE)
static const char *keys[]={"start-on-login","auto-disable-enabled","auto-accept"};
static void fetch_local(CcNodalixLocalSendPage *self);
static void cc_nodalix_localsend_favorite_changed(GtkToggleButton *button,CcNodalixLocalSendPage *self);
static void local_state_ready(GObject *source,GAsyncResult *result,gpointer data)
{
  CcNodalixLocalSendPage *self=data;g_autoptr(GError) error=NULL;
  g_autoptr(GVariant) value=g_dbus_proxy_call_finish(G_DBUS_PROXY(source),result,&error);self->fetching=FALSE;
  if(value){const char *text;g_variant_get(value,"(&s)",&text);g_autoptr(JsonParser) parser=json_parser_new();
    if(json_parser_load_from_data(parser,text,-1,&error)){
      JsonObject *state=json_node_get_object(json_parser_get_root(parser));self->loading=TRUE;
      gboolean installed=json_object_get_boolean_member(state,"installed"),ready=json_object_get_boolean_member(state,"ready");
      gtk_widget_set_sensitive(self->prefs,installed);
      gtk_widget_set_sensitive(GTK_WIDGET(self->enabled),ready);
      adw_switch_row_set_active(self->enabled,json_object_get_boolean_member(state,"enabled"));
      gtk_label_set_text(self->status,json_object_get_string_member(state,"message"));
      gtk_widget_set_sensitive(self->search,ready&&json_object_get_boolean_member(state,"enabled"));
      if(installed){JsonObject *config=json_object_get_object_member(state,"config");
        if(!self->alias_dirty)gtk_editable_set_text(GTK_EDITABLE(self->alias),json_object_get_string_member(config,"alias"));
        adw_action_row_set_subtitle(self->folder,json_object_get_string_member(config,"download-folder"));
        for(guint i=0;i<3;i++)adw_switch_row_set_active(self->options[i],json_object_get_boolean_member(config,keys[i]));
        gtk_spin_button_set_value(self->minutes,json_object_get_int_member(config,"auto-disable-minutes"));
        gtk_widget_set_sensitive(GTK_WIDGET(self->minutes),adw_switch_row_get_active(self->options[1]));
      }
      GtkWidget *child;while((child=gtk_widget_get_first_child(GTK_WIDGET(self->devices))))gtk_list_box_remove(self->devices,child);
      JsonArray *devices=json_object_get_array_member(state,"devices");
      for(guint i=0;i<json_array_get_length(devices);i++){
        JsonObject *peer=json_array_get_object_element(devices,i);GtkWidget *row=adw_action_row_new();
        adw_preferences_row_set_use_markup(ADW_PREFERENCES_ROW(row),FALSE);
        adw_preferences_row_set_title(ADW_PREFERENCES_ROW(row),json_object_get_string_member(peer,"alias"));
        if(json_object_has_member(peer,"deviceModel"))adw_action_row_set_subtitle(ADW_ACTION_ROW(row),json_object_get_string_member(peer,"deviceModel"));
        GtkWidget *favorite=gtk_toggle_button_new();
        gtk_button_set_icon_name(GTK_BUTTON(favorite),"starred-symbolic");gtk_widget_set_tooltip_text(favorite,"Dispositivo favorito");
        gtk_toggle_button_set_active(GTK_TOGGLE_BUTTON(favorite),json_object_get_boolean_member(peer,"favorite"));gtk_widget_set_valign(favorite,GTK_ALIGN_CENTER);
        g_object_set_data_full(G_OBJECT(favorite),"fingerprint",g_strdup(json_object_get_string_member(peer,"fingerprint")),g_free);
        g_signal_connect_object(favorite,"toggled",G_CALLBACK(cc_nodalix_localsend_favorite_changed),self,0);
        adw_action_row_add_suffix(ADW_ACTION_ROW(row),favorite);gtk_list_box_append(self->devices,row);
      }
      self->loading=FALSE;
    }
  }else if(!g_error_matches(error,G_IO_ERROR,G_IO_ERROR_CANCELLED))gtk_label_set_text(self->status,error->message);
  g_object_unref(self);
}
static void fetch_local(CcNodalixLocalSendPage *self)
{
  if(!self->proxy||self->fetching)return;self->fetching=TRUE;
  g_dbus_proxy_call(self->proxy,"GetLocalSend",NULL,G_DBUS_CALL_FLAGS_NONE,3000,self->cancel,local_state_ready,g_object_ref(self));
}
static gboolean poll_local(gpointer data){fetch_local(data);return G_SOURCE_CONTINUE;}
static void local_operation_done(GObject *source,GAsyncResult *result,gpointer data)
{
  CcNodalixLocalSendPage *self=data;g_autoptr(GError) error=NULL;g_autoptr(GVariant) value=g_dbus_proxy_call_finish(G_DBUS_PROXY(source),result,&error);
  if(!value&&!g_error_matches(error,G_IO_ERROR,G_IO_ERROR_CANCELLED))gtk_label_set_text(self->status,error->message);
  else fetch_local(self);g_object_unref(self);
}
static void local_call(CcNodalixLocalSendPage *self,const char *method,GVariant *args)
{
  if(!self->proxy||self->loading)return;
  g_dbus_proxy_call(self->proxy,method,args,G_DBUS_CALL_FLAGS_NONE,3000,self->cancel,local_operation_done,g_object_ref(self));
}
static void save_option(CcNodalixLocalSendPage *self,const char *key,const char *json)
{local_call(self,"SetLocalSendOption",g_variant_new("(ss)",key,json));}
static void cc_nodalix_localsend_favorite_changed(GtkToggleButton *button,CcNodalixLocalSendPage *self)
{local_call(self,"SetLocalSendFavorite",g_variant_new("(sb)",(char*)g_object_get_data(G_OBJECT(button),"fingerprint"),gtk_toggle_button_get_active(button)));}
static void local_enabled_changed(AdwSwitchRow *row,GParamSpec *pspec,CcNodalixLocalSendPage *self)
{local_call(self,"SetLocalSendEnabled",g_variant_new("(b)",adw_switch_row_get_active(row)));}
static void local_option_changed(AdwSwitchRow *row,GParamSpec *pspec,CcNodalixLocalSendPage *self)
{save_option(self,g_object_get_data(G_OBJECT(row),"key"),adw_switch_row_get_active(row)?"true":"false");}
static void local_alias_changed(GtkEditable *entry,CcNodalixLocalSendPage *self)
{if(!self->loading)self->alias_dirty=TRUE;}
static void local_alias_apply(AdwEntryRow *row,CcNodalixLocalSendPage *self)
{
  JsonNode *node=json_node_new(JSON_NODE_VALUE);json_node_set_string(node,gtk_editable_get_text(GTK_EDITABLE(row)));
  g_autofree char *json=json_to_string(node,FALSE);json_node_free(node);self->alias_dirty=FALSE;save_option(self,"alias",json);
}
static void local_minutes_changed(GtkSpinButton *button,CcNodalixLocalSendPage *self)
{g_autofree char *json=g_strdup_printf("%d",gtk_spin_button_get_value_as_int(button));save_option(self,"auto-disable-minutes",json);}
static void local_search_clicked(GtkButton *button,CcNodalixLocalSendPage *self)
{local_call(self,"RefreshLocalSend",NULL);}
static void local_folder_selected(GObject *source,GAsyncResult *result,gpointer data)
{
  CcNodalixLocalSendPage *self=data;g_autoptr(GError) error=NULL;g_autoptr(GFile) folder=gtk_file_dialog_select_folder_finish(GTK_FILE_DIALOG(source),result,&error);
  if(folder){g_autofree char *path=g_file_get_path(folder);if(path){JsonNode *node=json_node_new(JSON_NODE_VALUE);json_node_set_string(node,path);g_autofree char *json=json_to_string(node,FALSE);json_node_free(node);save_option(self,"download-folder",json);}}
  else if(!g_error_matches(error,GTK_DIALOG_ERROR,GTK_DIALOG_ERROR_DISMISSED)&&!g_error_matches(error,G_IO_ERROR,G_IO_ERROR_CANCELLED))gtk_label_set_text(self->status,error->message);
  g_object_unref(self);
}
static void local_folder_clicked(GtkButton *button,CcNodalixLocalSendPage *self)
{
  GtkFileDialog *dialog=gtk_file_dialog_new();gtk_file_dialog_set_title(dialog,"Carpeta de recepción de LocalSend");
  gtk_file_dialog_select_folder(dialog,GTK_WINDOW(gtk_widget_get_root(GTK_WIDGET(self))),self->cancel,local_folder_selected,g_object_ref(self));g_object_unref(dialog);
}
static void local_proxy_ready(GObject *source,GAsyncResult *result,gpointer data)
{
  CcNodalixLocalSendPage *self=data;g_autoptr(GError) error=NULL;self->proxy=g_dbus_proxy_new_for_bus_finish(result,&error);
  if(self->proxy)fetch_local(self);else if(!g_error_matches(error,G_IO_ERROR,G_IO_ERROR_CANCELLED))gtk_label_set_text(self->status,error->message);g_object_unref(self);
}
static void local_dispose(GObject *object)
{
  CcNodalixLocalSendPage *self=CC_NODALIX_LOCALSEND_PAGE(object);if(self->timer){g_source_remove(self->timer);self->timer=0;}
  if(self->cancel)g_cancellable_cancel(self->cancel);g_clear_object(&self->cancel);g_clear_object(&self->proxy);
  G_OBJECT_CLASS(cc_nodalix_localsend_page_parent_class)->dispose(object);
}
static void cc_nodalix_localsend_page_class_init(CcNodalixLocalSendPageClass *klass){G_OBJECT_CLASS(klass)->dispose=local_dispose;}
static void cc_nodalix_localsend_page_init(CcNodalixLocalSendPage *self)
{
  self->cancel=g_cancellable_new();adw_navigation_page_set_title(ADW_NAVIGATION_PAGE(self),"LocalSend");
  GtkWidget *toolbar=adw_toolbar_view_new();adw_toolbar_view_add_top_bar(ADW_TOOLBAR_VIEW(toolbar),adw_header_bar_new());
  GtkWidget *page=adw_preferences_page_new();adw_preferences_page_set_description(ADW_PREFERENCES_PAGE(page),"Nautilus y los ajustes rápidos comparten este mismo LocalSend, sus dispositivos y sus favoritos.");
  AdwPreferencesGroup *group=ADW_PREFERENCES_GROUP(adw_preferences_group_new());adw_preferences_group_set_title(group,"Compartir archivos");
  self->status=GTK_LABEL(gtk_label_new("Conectando con LocalSend…"));gtk_label_set_wrap(self->status,TRUE);adw_preferences_group_add(group,GTK_WIDGET(self->status));
  self->enabled=ADW_SWITCH_ROW(adw_switch_row_new());adw_preferences_row_set_title(ADW_PREFERENCES_ROW(self->enabled),"LocalSend activo");adw_preferences_row_set_use_markup(ADW_PREFERENCES_ROW(self->enabled),FALSE);gtk_widget_set_sensitive(GTK_WIDGET(self->enabled),FALSE);g_signal_connect_object(self->enabled,"notify::active",G_CALLBACK(local_enabled_changed),self,0);adw_preferences_group_add(group,GTK_WIDGET(self->enabled));
  self->search=gtk_button_new_with_label("Buscar dispositivos");g_signal_connect_object(self->search,"clicked",G_CALLBACK(local_search_clicked),self,0);gtk_widget_set_sensitive(self->search,FALSE);adw_preferences_group_add(group,self->search);adw_preferences_page_add(ADW_PREFERENCES_PAGE(page),group);
  group=ADW_PREFERENCES_GROUP(adw_preferences_group_new());self->prefs=GTK_WIDGET(group);adw_preferences_group_set_title(group,"Configuración");gtk_widget_set_sensitive(self->prefs,FALSE);
  self->alias=ADW_ENTRY_ROW(adw_entry_row_new());adw_preferences_row_set_title(ADW_PREFERENCES_ROW(self->alias),"Nombre de este equipo");adw_entry_row_set_show_apply_button(self->alias,TRUE);g_signal_connect_object(self->alias,"changed",G_CALLBACK(local_alias_changed),self,0);g_signal_connect_object(self->alias,"apply",G_CALLBACK(local_alias_apply),self,0);adw_preferences_group_add(group,GTK_WIDGET(self->alias));
  self->folder=ADW_ACTION_ROW(adw_action_row_new());adw_preferences_row_set_title(ADW_PREFERENCES_ROW(self->folder),"Carpeta de recepción");adw_preferences_row_set_use_markup(ADW_PREFERENCES_ROW(self->folder),FALSE);GtkWidget *choose=gtk_button_new_with_label("Elegir…");gtk_widget_set_valign(choose,GTK_ALIGN_CENTER);g_signal_connect_object(choose,"clicked",G_CALLBACK(local_folder_clicked),self,0);adw_action_row_add_suffix(self->folder,choose);adw_preferences_group_add(group,GTK_WIDGET(self->folder));
  const char *titles[]={"Activar al iniciar sesión","Desactivar automáticamente","Aceptar archivos automáticamente"};
  for(guint i=0;i<3;i++){self->options[i]=ADW_SWITCH_ROW(adw_switch_row_new());adw_preferences_row_set_title(ADW_PREFERENCES_ROW(self->options[i]),titles[i]);g_object_set_data(G_OBJECT(self->options[i]),"key",(gpointer)keys[i]);g_signal_connect_object(self->options[i],"notify::active",G_CALLBACK(local_option_changed),self,0);adw_preferences_group_add(group,GTK_WIDGET(self->options[i]));}
  adw_action_row_set_subtitle(ADW_ACTION_ROW(self->options[1]),"Desactívalo para mantener LocalSend siempre activo");
  adw_action_row_set_subtitle(ADW_ACTION_ROW(self->options[2]),"Los dispositivos de tu red podrán enviar sin confirmación");
  GtkWidget *row=adw_action_row_new();adw_preferences_row_set_title(ADW_PREFERENCES_ROW(row),"Minutos antes de desactivar");self->minutes=GTK_SPIN_BUTTON(gtk_spin_button_new_with_range(1,1440,1));gtk_widget_set_valign(GTK_WIDGET(self->minutes),GTK_ALIGN_CENTER);g_signal_connect_object(self->minutes,"value-changed",G_CALLBACK(local_minutes_changed),self,0);adw_action_row_add_suffix(ADW_ACTION_ROW(row),GTK_WIDGET(self->minutes));adw_preferences_group_add(group,row);adw_preferences_page_add(ADW_PREFERENCES_PAGE(page),group);
  group=ADW_PREFERENCES_GROUP(adw_preferences_group_new());adw_preferences_group_set_title(group,"Dispositivos cercanos");adw_preferences_group_set_description(group,"Marca con la estrella tus favoritos. Aparecerán primero también en Nautilus.");self->devices=GTK_LIST_BOX(gtk_list_box_new());gtk_list_box_set_selection_mode(self->devices,GTK_SELECTION_NONE);gtk_widget_add_css_class(GTK_WIDGET(self->devices),"boxed-list");adw_preferences_group_add(group,GTK_WIDGET(self->devices));adw_preferences_page_add(ADW_PREFERENCES_PAGE(page),group);
  adw_toolbar_view_set_content(ADW_TOOLBAR_VIEW(toolbar),page);adw_navigation_page_set_child(ADW_NAVIGATION_PAGE(self),toolbar);
  self->timer=g_timeout_add_seconds(3,poll_local,self);g_dbus_proxy_new_for_bus(G_BUS_TYPE_SESSION,G_DBUS_PROXY_FLAGS_NONE,NULL,BUS,OBJECT,IFACE,self->cancel,local_proxy_ready,g_object_ref(self));
}
