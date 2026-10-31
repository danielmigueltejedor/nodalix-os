/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "cc-nodalix-pages.h"

struct _CcNodalixDiagnosticsPage {
  AdwNavigationPage parent_instance;
  GCancellable *cancel;
  GSubprocess *process;
  GtkTextBuffer *report;
  GtkWidget *refresh;
  GtkWidget *copy;
  GtkSpinner *spinner;
};
G_DEFINE_TYPE(CcNodalixDiagnosticsPage,cc_nodalix_diagnostics_page,ADW_TYPE_NAVIGATION_PAGE)

static void diagnostics_ready(GObject *source,GAsyncResult *result,gpointer data)
{
  CcNodalixDiagnosticsPage *self=data;
  g_autoptr(GError) error=NULL;
  g_autofree char *text=NULL;
  g_autofree char *errors=NULL;
  gboolean ok=g_subprocess_communicate_utf8_finish(G_SUBPROCESS(source),result,&text,&errors,&error);
  if (!g_cancellable_is_cancelled(self->cancel)) {
    ok=ok && g_subprocess_get_successful(G_SUBPROCESS(source));
    gtk_text_buffer_set_text(self->report,ok ? text : "No se pudo generar el informe. Revisa que nodalix-integrations esté instalado.",-1);
    gtk_widget_set_sensitive(self->refresh,TRUE);
    gtk_widget_set_sensitive(self->copy,ok);
    gtk_spinner_stop(self->spinner);
  }
  g_clear_object(&self->process);
  g_object_unref(self);
}

static void diagnostics_refresh(GtkButton *button,CcNodalixDiagnosticsPage *self)
{
  if (self->process) return;
  g_autoptr(GError) error=NULL;
  self->process=g_subprocess_new(G_SUBPROCESS_FLAGS_STDOUT_PIPE|G_SUBPROCESS_FLAGS_STDERR_PIPE,
      &error,"/usr/bin/nodalix-diagnostics","--json",NULL);
  if (!self->process) {
    gtk_text_buffer_set_text(self->report,"Instala nodalix-integrations para generar el diagnóstico.",-1);
    return;
  }
  gtk_widget_set_sensitive(self->refresh,FALSE);
  gtk_widget_set_sensitive(self->copy,FALSE);
  gtk_spinner_start(self->spinner);
  g_subprocess_communicate_utf8_async(self->process,NULL,self->cancel,diagnostics_ready,g_object_ref(self));
}

static void diagnostics_copy(GtkButton *button,CcNodalixDiagnosticsPage *self)
{
  GtkTextIter start,end;
  gtk_text_buffer_get_bounds(self->report,&start,&end);
  g_autofree char *text=gtk_text_buffer_get_text(self->report,&start,&end,FALSE);
  gdk_clipboard_set_text(gtk_widget_get_clipboard(GTK_WIDGET(self)),text);
}

static void diagnostics_dispose(GObject *object)
{
  CcNodalixDiagnosticsPage *self=CC_NODALIX_DIAGNOSTICS_PAGE(object);
  if(self->cancel)g_cancellable_cancel(self->cancel);
  if(self->process)g_subprocess_force_exit(self->process);
  g_clear_object(&self->cancel);g_clear_object(&self->process);
  G_OBJECT_CLASS(cc_nodalix_diagnostics_page_parent_class)->dispose(object);
}

static void cc_nodalix_diagnostics_page_class_init(CcNodalixDiagnosticsPageClass *klass)
{ G_OBJECT_CLASS(klass)->dispose=diagnostics_dispose; }

static void cc_nodalix_diagnostics_page_init(CcNodalixDiagnosticsPage *self)
{
  self->cancel=g_cancellable_new();
  adw_navigation_page_set_title(ADW_NAVIGATION_PAGE(self),"Diagnóstico del sistema");
  GtkWidget *toolbar=adw_toolbar_view_new();
  adw_toolbar_view_add_top_bar(ADW_TOOLBAR_VIEW(toolbar),adw_header_bar_new());
  GtkWidget *page=adw_preferences_page_new();
  AdwPreferencesGroup *group=ADW_PREFERENCES_GROUP(adw_preferences_group_new());
  adw_preferences_group_set_title(group,"Informe local de soporte");
  adw_preferences_group_set_description(group,"Versiones, memoria, pantallas, energía y servicios. No incluye nombres de cuenta, direcciones de red, identificadores de dispositivos ni contenido personal. Solo se comparte si lo copias tú.");
  GtkWidget *buttons=gtk_box_new(GTK_ORIENTATION_HORIZONTAL,12);
  self->refresh=gtk_button_new_with_label("Actualizar informe");
  self->copy=gtk_button_new_with_label("Copiar informe");
  gtk_widget_set_sensitive(self->copy,FALSE);
  self->spinner=GTK_SPINNER(gtk_spinner_new());
  gtk_box_append(GTK_BOX(buttons),self->refresh);gtk_box_append(GTK_BOX(buttons),self->copy);
  gtk_box_append(GTK_BOX(buttons),GTK_WIDGET(self->spinner));
  g_signal_connect(self->refresh,"clicked",G_CALLBACK(diagnostics_refresh),self);
  g_signal_connect(self->copy,"clicked",G_CALLBACK(diagnostics_copy),self);
  adw_preferences_group_add(group,buttons);
  GtkWidget *scroll=gtk_scrolled_window_new();
  gtk_scrolled_window_set_min_content_height(GTK_SCROLLED_WINDOW(scroll),320);
  GtkWidget *text=gtk_text_view_new();
  gtk_text_view_set_editable(GTK_TEXT_VIEW(text),FALSE);gtk_text_view_set_monospace(GTK_TEXT_VIEW(text),TRUE);
  gtk_text_view_set_wrap_mode(GTK_TEXT_VIEW(text),GTK_WRAP_WORD_CHAR);
  self->report=gtk_text_view_get_buffer(GTK_TEXT_VIEW(text));
  gtk_scrolled_window_set_child(GTK_SCROLLED_WINDOW(scroll),text);
  adw_preferences_group_add(group,scroll);adw_preferences_page_add(ADW_PREFERENCES_PAGE(page),group);
  adw_toolbar_view_set_content(ADW_TOOLBAR_VIEW(toolbar),page);
  adw_navigation_page_set_child(ADW_NAVIGATION_PAGE(self),toolbar);
  diagnostics_refresh(NULL,self);
}
