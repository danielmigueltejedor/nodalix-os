/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "cc-nodalix-background.h"
#include <json-glib/json-glib.h>

#define BUS "com.nodalix.Settings"
#define OBJECT "/com/nodalix/Settings"
#define IFACE "com.nodalix.Settings1"
#define WIDTH 144
#define HEIGHT 108

struct _CcNodalixBackgroundChooser {
  GtkBox parent_instance;
  GDBusProxy *proxy;
  GCancellable *cancel;
  GtkFlowBox *gallery;
  GtkLabel *status;
  guint timer;
  gboolean fetching, busy, operation_error;
};
G_DEFINE_TYPE (CcNodalixBackgroundChooser, cc_nodalix_background_chooser, GTK_TYPE_BOX)

static void fetch_state (CcNodalixBackgroundChooser *self);
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
      for (int i=0;;i++) {
        GtkFlowBoxChild *child=gtk_flow_box_get_child_at_index(self->gallery,i);
        if (!child) break;
        gboolean active=selected&&g_strcmp0(path,g_object_get_data(G_OBJECT(child),"path"))==0;
        if (active) gtk_widget_add_css_class(GTK_WIDGET(child),"active-item");
        else gtk_widget_remove_css_class(GTK_WIDGET(child),"active-item");
        gtk_accessible_update_state(GTK_ACCESSIBLE(child),GTK_ACCESSIBLE_STATE_CHECKED,active,-1);
      }
      gtk_widget_set_sensitive(GTK_WIDGET(self->gallery),!self->busy&&json_object_get_boolean_member(state,"available"));
    }
  } else if (!g_error_matches(error,G_IO_ERROR,G_IO_ERROR_CANCELLED)) {
    g_dbus_error_strip_remote_error(error);
    gtk_label_set_text(self->status,error->message);
  }
  g_object_unref(self);
}
static void fetch_state (CcNodalixBackgroundChooser *self)
{
  if (!self->proxy||self->fetching) return;
  self->fetching=TRUE;
  g_dbus_proxy_call(self->proxy,"GetWallpaperState",NULL,G_DBUS_CALL_FLAGS_NONE,4000,self->cancel,state_ready,g_object_ref(self));
}
static gboolean poll_state (gpointer data) {fetch_state(data);return G_SOURCE_CONTINUE;}
static void wallpaper_done (GObject *source,GAsyncResult *result,gpointer data)
{
  CcNodalixBackgroundChooser *self=data;g_autoptr(GError) error=NULL;
  g_autoptr(GVariant) value=g_dbus_proxy_call_finish(G_DBUS_PROXY(source),result,&error);
  self->busy=FALSE;
  if (!value&&!g_error_matches(error,G_IO_ERROR,G_IO_ERROR_CANCELLED)) {
    g_dbus_error_strip_remote_error(error);gtk_label_set_text(self->status,error->message);self->operation_error=TRUE;
  }
  fetch_state(self);g_object_unref(self);
}
static void thumbnail_activated (GtkFlowBox *box,GtkFlowBoxChild *child,CcNodalixBackgroundChooser *self)
{
  if (!self->proxy||self->busy) return;
  self->busy=TRUE;self->operation_error=FALSE;
  gtk_label_set_text(self->status,"Activando fondo animado…");
  gtk_widget_set_sensitive(GTK_WIDGET(self->gallery),FALSE);
  const char *path=g_object_get_data(G_OBJECT(child),"path");
  g_dbus_proxy_call(self->proxy,"SetWallpaper",g_variant_new("(s)",path),G_DBUS_CALL_FLAGS_NONE,5000,self->cancel,wallpaper_done,g_object_ref(self));
}
static GdkTexture *poster_texture (const char *path)
{
  g_autoptr(GdkPixbuf) original=gdk_pixbuf_new_from_file(path,NULL);
  if (!original) return NULL;
  double scale=MAX((double)WIDTH/gdk_pixbuf_get_width(original),(double)HEIGHT/gdk_pixbuf_get_height(original));
  int width=MAX(WIDTH,(int)(gdk_pixbuf_get_width(original)*scale));
  int height=MAX(HEIGHT,(int)(gdk_pixbuf_get_height(original)*scale));
  g_autoptr(GdkPixbuf) scaled=gdk_pixbuf_scale_simple(original,width,height,GDK_INTERP_BILINEAR);
  g_autoptr(GdkPixbuf) cropped=gdk_pixbuf_new_subpixbuf(scaled,(width-WIDTH)/2,(height-HEIGHT)/2,WIDTH,HEIGHT);
  return gdk_texture_new_for_pixbuf(cropped);
}
static void gallery_ready (GObject *source,GAsyncResult *result,gpointer data)
{
  CcNodalixBackgroundChooser *self=data;g_autoptr(GError) error=NULL;
  g_autoptr(GVariant) value=g_dbus_proxy_call_finish(G_DBUS_PROXY(source),result,&error);
  if (value) {
    const char *text;g_variant_get(value,"(&s)",&text);g_autoptr(JsonParser) parser=json_parser_new();
    if (json_parser_load_from_data(parser,text,-1,&error)) {
      JsonArray *items=json_node_get_array(json_parser_get_root(parser));
      for(guint i=0;i<json_array_get_length(items);i++) {
        JsonObject *item=json_array_get_object_element(items,i);
        g_autoptr(GdkTexture) texture=poster_texture(json_object_get_string_member(item,"preview"));
        GtkWidget *picture=gtk_picture_new_for_paintable(texture?GDK_PAINTABLE(texture):NULL);
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
        GtkWidget *child=gtk_flow_box_child_new();gtk_flow_box_child_set_child(GTK_FLOW_BOX_CHILD(child),overlay);
        gtk_widget_set_halign(child,GTK_ALIGN_CENTER);gtk_widget_set_valign(child,GTK_ALIGN_CENTER);
        gtk_widget_set_tooltip_text(child,json_object_get_string_member(item,"title"));
        g_object_set(child,"accessible-role",GTK_ACCESSIBLE_ROLE_TOGGLE_BUTTON,NULL);
        gtk_accessible_update_property(GTK_ACCESSIBLE(child),GTK_ACCESSIBLE_PROPERTY_LABEL,json_object_get_string_member(item,"title"),-1);
        gtk_accessible_update_state(GTK_ACCESSIBLE(child),GTK_ACCESSIBLE_STATE_CHECKED,FALSE,-1);
        g_object_set_data_full(G_OBJECT(child),"path",g_strdup(json_object_get_string_member(item,"path")),g_free);
        gtk_flow_box_insert(self->gallery,child,-1);
      }
      fetch_state(self);
    }
  } else if (!g_error_matches(error,G_IO_ERROR,G_IO_ERROR_CANCELLED)) {
    g_dbus_error_strip_remote_error(error);gtk_label_set_text(self->status,error->message);
  }
  g_object_unref(self);
}
static void proxy_ready (GObject *source,GAsyncResult *result,gpointer data)
{
  CcNodalixBackgroundChooser *self=data;g_autoptr(GError) error=NULL;
  self->proxy=g_dbus_proxy_new_for_bus_finish(result,&error);
  if (self->proxy) g_dbus_proxy_call(self->proxy,"GetWallpapers",NULL,G_DBUS_CALL_FLAGS_NONE,4000,self->cancel,gallery_ready,g_object_ref(self));
  else if (!g_error_matches(error,G_IO_ERROR,G_IO_ERROR_CANCELLED)) gtk_label_set_text(self->status,error->message);
  g_object_unref(self);
}
static void chooser_dispose (GObject *object)
{
  CcNodalixBackgroundChooser *self=CC_NODALIX_BACKGROUND_CHOOSER(object);
  if(self->timer){g_source_remove(self->timer);self->timer=0;}
  if(self->cancel)g_cancellable_cancel(self->cancel);
  g_clear_object(&self->cancel);g_clear_object(&self->proxy);
  G_OBJECT_CLASS(cc_nodalix_background_chooser_parent_class)->dispose(object);
}
static void cc_nodalix_background_chooser_class_init (CcNodalixBackgroundChooserClass *klass)
{G_OBJECT_CLASS(klass)->dispose=chooser_dispose;}
static void cc_nodalix_background_chooser_init (CcNodalixBackgroundChooser *self)
{
  self->cancel=g_cancellable_new();gtk_orientable_set_orientation(GTK_ORIENTABLE(self),GTK_ORIENTATION_VERTICAL);
  self->gallery=GTK_FLOW_BOX(gtk_flow_box_new());
  gtk_widget_add_css_class(GTK_WIDGET(self->gallery),"background-flowbox");
  gtk_flow_box_set_homogeneous(self->gallery,TRUE);gtk_flow_box_set_selection_mode(self->gallery,GTK_SELECTION_SINGLE);
  gtk_flow_box_set_min_children_per_line(self->gallery,1);gtk_flow_box_set_max_children_per_line(self->gallery,8);
  gtk_flow_box_set_row_spacing(self->gallery,12);gtk_flow_box_set_column_spacing(self->gallery,12);
  gtk_flow_box_set_activate_on_single_click(self->gallery,TRUE);gtk_widget_set_halign(GTK_WIDGET(self->gallery),GTK_ALIGN_CENTER);
  GtkWidget *widgets[]={GTK_WIDGET(self->gallery),NULL};
  self->status=GTK_LABEL(gtk_label_new("Cargando fondos animados…"));gtk_label_set_wrap(self->status,TRUE);gtk_label_set_xalign(self->status,0);
  gtk_widget_add_css_class(GTK_WIDGET(self->status),"dim-label");widgets[1]=GTK_WIDGET(self->status);
  for(guint i=0;i<2;i++){gtk_widget_set_margin_start(widgets[i],12);gtk_widget_set_margin_end(widgets[i],12);gtk_widget_set_margin_top(widgets[i],12);gtk_widget_set_margin_bottom(widgets[i],12);gtk_box_append(GTK_BOX(self),widgets[i]);}
  g_signal_connect_object(self->gallery,"child-activated",G_CALLBACK(thumbnail_activated),self,0);
  self->timer=g_timeout_add_seconds(3,poll_state,self);
  g_dbus_proxy_new_for_bus(G_BUS_TYPE_SESSION,G_DBUS_PROXY_FLAGS_NONE,NULL,BUS,OBJECT,IFACE,self->cancel,proxy_ready,g_object_ref(self));
}
