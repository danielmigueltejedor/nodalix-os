/* nautilus-nodalix-localsend.c
 *
 * "Compartir con LocalSend": send the selection to a nearby device from the
 * context menu. Built into Archivador; the behaviour is that of the nautilus-python
 * extension it replaces (gnome/nautilus/nodalix_localsend.py).
 *
 * The device list comes from the session service com.nodalix.LocalSend. It is
 * cached and refreshed asynchronously, so opening a menu never waits on D-Bus.
 * Sending runs in a transient user unit, independent from the file manager.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#include <gio/gio.h>
#include <glib.h>
#include <glib-object.h>
#include <json-glib/json-glib.h>
#include <nautilus-extension.h>

#define BUS_NAME "com.nodalix.LocalSend"
#define OBJECT_PATH "/com/nodalix/LocalSend"
#define INTERFACE_NAME "com.nodalix.LocalSend1"
#define SEND_HELPER "/usr/lib/nodalix/localsend-send"
#define NOTIFICATION_ICON "send-to-symbolic"
#define CACHE_SECONDS 8
#define REQUEST_TIMEOUT_MS 2000

#define NODALIX_TYPE_LOCALSEND_PROVIDER (nodalix_localsend_provider_get_type ())
G_DECLARE_FINAL_TYPE (NodalixLocalsendProvider, nodalix_localsend_provider,
                      NODALIX, LOCALSEND_PROVIDER, GObject)

typedef struct
{
    char *fingerprint;
    char *alias;
    char *model;
    gboolean favorite;
} Device;

struct _NodalixLocalsendProvider
{
    GObject parent_instance;

    GDBusConnection *bus;
    guint changed_subscription;
    guint owner_subscription;
    GPtrArray *devices;       /* of Device*, owned */
    gint64 cache_time;        /* monotonic microseconds, 0 when never filled */
    gboolean refreshing;
    GCancellable *cancellable;
};

static void nodalix_localsend_provider_menu_provider_init (NautilusMenuProviderInterface *iface);

G_DEFINE_DYNAMIC_TYPE_EXTENDED (NodalixLocalsendProvider, nodalix_localsend_provider, G_TYPE_OBJECT, 0,
                                G_IMPLEMENT_INTERFACE_DYNAMIC (NAUTILUS_TYPE_MENU_PROVIDER,
                                                               nodalix_localsend_provider_menu_provider_init))

static void
device_free (Device *device)
{
    g_free (device->fingerprint);
    g_free (device->alias);
    g_free (device->model);
    g_free (device);
}

static GPtrArray *
devices_new (void)
{
    return g_ptr_array_new_with_free_func ((GDestroyNotify) device_free);
}

static gboolean
devices_equal (GPtrArray *a,
               GPtrArray *b)
{
    if (a->len != b->len)
    {
        return FALSE;
    }
    for (guint i = 0; i < a->len; i++)
    {
        Device *x = g_ptr_array_index (a, i);
        Device *y = g_ptr_array_index (b, i);

        if (g_strcmp0 (x->fingerprint, y->fingerprint) != 0 ||
            g_strcmp0 (x->alias, y->alias) != 0 ||
            g_strcmp0 (x->model, y->model) != 0 ||
            x->favorite != y->favorite)
        {
            return FALSE;
        }
    }
    return TRUE;
}

static void
notify (const char *title,
        const char *body,
        const char *urgency)
{
    g_autofree char *icon = g_strdup_printf ("--icon=%s", NOTIFICATION_ICON);
    g_autofree char *level = g_strdup_printf ("--urgency=%s", urgency);
    const char *argv[] = { "notify-send", "--app-name=Nodalix LocalSend", icon, level, title, body, NULL };

    g_spawn_async (NULL, (char **) argv, NULL,
                   G_SPAWN_SEARCH_PATH | G_SPAWN_STDOUT_TO_DEV_NULL | G_SPAWN_STDERR_TO_DEV_NULL,
                   NULL, NULL, NULL, NULL);
}

/* Parses the JSON of GetStatus. Returns NULL when it is malformed; an empty
 * array when the service is disabled. */
static GPtrArray *
parse_status (const char *json)
{
    g_autoptr (JsonParser) parser = json_parser_new ();
    g_autoptr (GPtrArray) devices = devices_new ();
    JsonNode *root;
    JsonObject *status;
    JsonArray *list;

    if (!json_parser_load_from_data (parser, json, -1, NULL))
    {
        return NULL;
    }
    root = json_parser_get_root (parser);
    if (root == NULL || !JSON_NODE_HOLDS_OBJECT (root))
    {
        return NULL;
    }
    status = json_node_get_object (root);
    if (!json_object_has_member (status, "enabled") ||
        !json_object_get_boolean_member (status, "enabled") ||
        !json_object_has_member (status, "devices"))
    {
        return g_steal_pointer (&devices);
    }
    list = json_object_get_array_member (status, "devices");
    if (list == NULL)
    {
        return g_steal_pointer (&devices);
    }

    for (guint i = 0; i < json_array_get_length (list); i++)
    {
        JsonObject *item = json_array_get_object_element (list, i);
        Device *device;

        if (item == NULL)
        {
            continue;
        }
        device = g_new0 (Device, 1);
        device->fingerprint = g_strdup (json_object_has_member (item, "fingerprint") ?
                                        json_object_get_string_member_with_default (item, "fingerprint", "") : "");
        device->alias = g_strdup (json_object_get_string_member_with_default (item, "alias", ""));
        device->model = g_strdup (json_object_get_string_member_with_default (item, "deviceModel", ""));
        device->favorite = json_object_get_boolean_member_with_default (item, "favorite", FALSE);
        g_ptr_array_add (devices, device);
    }

    return g_steal_pointer (&devices);
}

static void
status_ready (GObject      *source,
              GAsyncResult *result,
              gpointer      user_data)
{
    NodalixLocalsendProvider *self = user_data;
    g_autoptr (GError) error = NULL;
    g_autoptr (GVariant) reply = g_dbus_connection_call_finish (G_DBUS_CONNECTION (source), result, &error);
    g_autoptr (GPtrArray) devices = NULL;

    if (error != NULL && g_error_matches (error, G_IO_ERROR, G_IO_ERROR_CANCELLED))
    {
        return;
    }

    self->refreshing = FALSE;
    if (reply != NULL)
    {
        const char *json = NULL;

        g_variant_get (reply, "(&s)", &json);
        devices = parse_status (json);
    }
    if (devices == NULL)
    {
        /* The service did not answer: keep what the menu already shows. */
        return;
    }

    self->cache_time = g_get_monotonic_time ();
    if (!devices_equal (self->devices, devices))
    {
        g_clear_pointer (&self->devices, g_ptr_array_unref);
        self->devices = g_steal_pointer (&devices);
        /* An already open menu must be rebuilt when discovery completes. */
        nautilus_menu_provider_emit_items_updated_signal (NAUTILUS_MENU_PROVIDER (self));
    }
}

static void
refresh_devices (NodalixLocalsendProvider *self,
                 gboolean                 force)
{
    gboolean fresh = self->cache_time != 0 &&
                     g_get_monotonic_time () - self->cache_time < CACHE_SECONDS * G_USEC_PER_SEC;

    if (self->bus == NULL || self->refreshing || (fresh && !force))
    {
        return;
    }
    self->refreshing = TRUE;
    g_dbus_connection_call (self->bus, BUS_NAME, OBJECT_PATH, INTERFACE_NAME, "GetStatus", NULL,
                            G_VARIANT_TYPE ("(s)"), G_DBUS_CALL_FLAGS_NONE, REQUEST_TIMEOUT_MS,
                            self->cancellable, status_ready, self);
}

static void
service_changed (GDBusConnection *connection,
                 const char      *sender,
                 const char      *path,
                 const char      *interface,
                 const char      *signal,
                 GVariant        *parameters,
                 gpointer         user_data)
{
    refresh_devices (user_data, TRUE);
}

static void
spawn_detached (const char * const *argv)
{
    g_autoptr (GError) error = NULL;

    if (!g_spawn_async (NULL, (char **) argv, NULL,
                        G_SPAWN_SEARCH_PATH | G_SPAWN_STDOUT_TO_DEV_NULL | G_SPAWN_STDERR_TO_DEV_NULL,
                        NULL, NULL, NULL, &error))
    {
        notify ("No se pudo iniciar el envío", error->message, "critical");
    }
}

static void
send_selection (NautilusMenuItem *item,
                gpointer          user_data)
{
    char **paths = g_object_get_data (G_OBJECT (item), "nodalix-paths");
    const char *fingerprint = g_object_get_data (G_OBJECT (item), "nodalix-fingerprint");
    const char *alias = g_object_get_data (G_OBJECT (item), "nodalix-alias");
    g_autoptr (GPtrArray) helper = g_ptr_array_new ();
    g_autoptr (GPtrArray) command = g_ptr_array_new ();
    g_autofree char *unit = NULL;
    g_autofree char *unit_arg = NULL;
    g_autofree char *id = g_uuid_string_random ();
    g_autoptr (GError) error = NULL;
    gint status = 0;

    if (paths == NULL || fingerprint == NULL || alias == NULL)
    {
        return;
    }

    g_ptr_array_add (helper, (gpointer) SEND_HELPER);
    g_ptr_array_add (helper, (gpointer) "--fingerprint");
    g_ptr_array_add (helper, (gpointer) fingerprint);
    g_ptr_array_add (helper, (gpointer) "--alias");
    g_ptr_array_add (helper, (gpointer) alias);
    g_ptr_array_add (helper, (gpointer) "--");
    for (char **p = paths; *p != NULL; p++)
    {
        g_ptr_array_add (helper, *p);
    }

    /* A transient user unit is independent from the file manager: a thread of
     * this process could disappear right after the menu callback. */
    id[12] = '\0';
    unit = g_strdup_printf ("nodalix-localsend-send-%s", id);
    unit_arg = g_strdup_printf ("--unit=%s", unit);
    g_ptr_array_add (command, (gpointer) "systemd-run");
    g_ptr_array_add (command, (gpointer) "--user");
    g_ptr_array_add (command, (gpointer) "--collect");
    g_ptr_array_add (command, (gpointer) "--quiet");
    g_ptr_array_add (command, unit_arg);
    g_ptr_array_add (command, (gpointer) "--");
    for (guint i = 0; i < helper->len; i++)
    {
        g_ptr_array_add (command, g_ptr_array_index (helper, i));
    }
    g_ptr_array_add (command, NULL);

    if (g_spawn_sync (NULL, (char **) command->pdata, NULL,
                      G_SPAWN_SEARCH_PATH | G_SPAWN_STDOUT_TO_DEV_NULL | G_SPAWN_STDERR_TO_DEV_NULL,
                      NULL, NULL, NULL, NULL, &status, &error) &&
        g_spawn_check_wait_status (status, NULL))
    {
        return;
    }

    /* Keep a direct detached fallback for systems without systemd-run. */
    g_ptr_array_add (helper, NULL);
    spawn_detached ((const char * const *) helper->pdata);
}

static void
open_settings (NautilusMenuItem *item,
               gpointer          user_data)
{
    const char *argv[] = { "gnome-control-center", "sharing", "localsend", NULL };
    g_autoptr (GError) error = NULL;

    if (!g_spawn_async (NULL, (char **) argv, NULL,
                        G_SPAWN_SEARCH_PATH | G_SPAWN_STDOUT_TO_DEV_NULL | G_SPAWN_STDERR_TO_DEV_NULL,
                        NULL, NULL, NULL, &error))
    {
        notify ("No se pudieron abrir los ajustes de LocalSend", error->message, "normal");
    }
}

static gboolean
refresh_after_announce (gpointer user_data)
{
    NodalixLocalsendProvider *self = user_data;

    refresh_devices (self, TRUE);
    notify ("Buscando dispositivos", "Vuelve a abrir el menú en unos segundos", "normal");
    g_object_unref (self);
    return G_SOURCE_REMOVE;
}

static void
announce_ready (GObject      *source,
                GAsyncResult *result,
                gpointer      user_data)
{
    NodalixLocalsendProvider *self = user_data;
    g_autoptr (GError) error = NULL;
    g_autoptr (GVariant) reply = g_dbus_connection_call_finish (G_DBUS_CONNECTION (source), result, &error);

    if (reply == NULL)
    {
        notify ("LocalSend no está disponible", "El servicio de Nodalix no está respondiendo", "critical");
        g_object_unref (self);
        return;
    }
    /* Allow multicast replies to arrive before the submenu cache is refreshed. */
    g_timeout_add (700, refresh_after_announce, self);
}

static void
search_devices (NautilusMenuItem *item,
                gpointer          user_data)
{
    NodalixLocalsendProvider *self = user_data;

    if (self->bus == NULL)
    {
        notify ("LocalSend no está disponible", "El servicio de Nodalix no está respondiendo", "critical");
        return;
    }
    g_dbus_connection_call (self->bus, BUS_NAME, OBJECT_PATH, INTERFACE_NAME, "Refresh", NULL, NULL,
                            G_DBUS_CALL_FLAGS_NONE, REQUEST_TIMEOUT_MS, NULL,
                            announce_ready, g_object_ref (self));
}

static GList *
nodalix_localsend_get_file_items (NautilusMenuProvider *provider,
                                  GList                *files)
{
    NodalixLocalsendProvider *self = NODALIX_LOCALSEND_PROVIDER (provider);
    g_autoptr (GPtrArray) paths = g_ptr_array_new_with_free_func (g_free);
    NautilusMenuItem *root;
    g_autoptr (NautilusMenu) submenu = nautilus_menu_new ();
    NautilusMenuItem *settings;

    for (GList *l = files; l != NULL; l = l->next)
    {
        g_autoptr (GFile) location = nautilus_file_info_get_location (NAUTILUS_FILE_INFO (l->data));
        g_autofree char *path = location != NULL ? g_file_get_path (location) : NULL;

        if (path != NULL && g_file_test (path, G_FILE_TEST_EXISTS))
        {
            g_ptr_array_add (paths, g_steal_pointer (&path));
        }
    }
    if (paths->len == 0)
    {
        return NULL;
    }
    g_ptr_array_add (paths, NULL);

    root = nautilus_menu_item_new ("NodalixLocalSend::Share", "Compartir con LocalSend",
                                   "Enviar directamente a un dispositivo cercano", NOTIFICATION_ICON);

    /* Discovery signals update the cache and invalidate the menus. */
    refresh_devices (self, FALSE);

    if (self->devices->len > 0)
    {
        for (guint i = 0; i < self->devices->len; i++)
        {
            Device *device = g_ptr_array_index (self->devices, i);
            const char *alias = device->alias[0] != '\0' ? device->alias : "Dispositivo LocalSend";
            g_autofree char *name = g_strdup_printf ("NodalixLocalSend::Device%u", i);
            g_autofree char *label = NULL;
            g_autofree char *tip = g_strdup_printf ("Enviar la selección a %s", alias);
            NautilusMenuItem *receiver;

            if (device->model[0] == '\0' || g_ascii_strcasecmp (device->model, alias) == 0)
            {
                label = g_strdup (alias);
            }
            else
            {
                label = g_strdup_printf ("%s · %s", alias, device->model);
            }
            if (device->favorite)
            {
                g_autofree char *starred = g_strdup_printf ("★ %s", label);

                g_free (label);
                label = g_steal_pointer (&starred);
            }

            receiver = nautilus_menu_item_new (name, label, tip, NOTIFICATION_ICON);
            g_object_set_data_full (G_OBJECT (receiver), "nodalix-paths",
                                    g_strdupv ((char **) paths->pdata), (GDestroyNotify) g_strfreev);
            g_object_set_data_full (G_OBJECT (receiver), "nodalix-fingerprint",
                                    g_strdup (device->fingerprint), g_free);
            g_object_set_data_full (G_OBJECT (receiver), "nodalix-alias", g_strdup (alias), g_free);
            g_signal_connect (receiver, "activate", G_CALLBACK (send_selection), NULL);
            nautilus_menu_append_item (submenu, receiver);
            g_object_unref (receiver);
        }
    }
    else
    {
        NautilusMenuItem *empty = nautilus_menu_item_new ("NodalixLocalSend::NoDevices",
                                                          "No hay dispositivos disponibles",
                                                          "Abre LocalSend en el dispositivo receptor y vuelve a intentarlo",
                                                          NULL);
        NautilusMenuItem *search = nautilus_menu_item_new ("NodalixLocalSend::Refresh",
                                                           "Buscar dispositivos",
                                                           "Anunciar Nodalix de nuevo en la red local",
                                                           NULL);

        g_object_set (empty, "sensitive", FALSE, NULL);
        nautilus_menu_append_item (submenu, empty);
        g_signal_connect (search, "activate", G_CALLBACK (search_devices), self);
        nautilus_menu_append_item (submenu, search);
        g_object_unref (empty);
        g_object_unref (search);
    }

    settings = nautilus_menu_item_new ("NodalixLocalSend::Settings", "Ajustes de LocalSend",
                                       "Configurar el mismo LocalSend del sistema",
                                       "preferences-system-symbolic");
    g_signal_connect (settings, "activate", G_CALLBACK (open_settings), NULL);
    nautilus_menu_append_item (submenu, settings);
    g_object_unref (settings);

    nautilus_menu_item_set_submenu (root, submenu);
    return g_list_append (NULL, root);
}

static GList *
nodalix_localsend_get_background_items (NautilusMenuProvider *provider,
                                        NautilusFileInfo     *current_folder)
{
    return NULL;
}

static void
nodalix_localsend_provider_menu_provider_init (NautilusMenuProviderInterface *iface)
{
    iface->get_file_items = nodalix_localsend_get_file_items;
    iface->get_background_items = nodalix_localsend_get_background_items;
}

static void
nodalix_localsend_provider_finalize (GObject *object)
{
    NodalixLocalsendProvider *self = NODALIX_LOCALSEND_PROVIDER (object);

    g_cancellable_cancel (self->cancellable);
    if (self->bus != NULL)
    {
        g_dbus_connection_signal_unsubscribe (self->bus, self->changed_subscription);
        g_dbus_connection_signal_unsubscribe (self->bus, self->owner_subscription);
    }
    g_clear_object (&self->bus);
    g_clear_object (&self->cancellable);
    g_clear_pointer (&self->devices, g_ptr_array_unref);

    G_OBJECT_CLASS (nodalix_localsend_provider_parent_class)->finalize (object);
}

static void
nodalix_localsend_provider_class_init (NodalixLocalsendProviderClass *klass)
{
    G_OBJECT_CLASS (klass)->finalize = nodalix_localsend_provider_finalize;
}

static void
nodalix_localsend_provider_class_finalize (NodalixLocalsendProviderClass *klass)
{
}

static void
nodalix_localsend_provider_init (NodalixLocalsendProvider *self)
{
    g_autoptr (GError) error = NULL;

    self->devices = devices_new ();
    self->cancellable = g_cancellable_new ();
    self->bus = g_bus_get_sync (G_BUS_TYPE_SESSION, NULL, &error);
    if (self->bus == NULL)
    {
        g_warning ("Nodalix LocalSend: no session bus: %s", error->message);
        return;
    }

    self->changed_subscription = g_dbus_connection_signal_subscribe (self->bus, BUS_NAME, INTERFACE_NAME,
                                                                     "Changed", OBJECT_PATH, NULL,
                                                                     G_DBUS_SIGNAL_FLAGS_NONE,
                                                                     service_changed, self, NULL);
    self->owner_subscription = g_dbus_connection_signal_subscribe (self->bus, "org.freedesktop.DBus",
                                                                   "org.freedesktop.DBus", "NameOwnerChanged",
                                                                   "/org/freedesktop/DBus", BUS_NAME,
                                                                   G_DBUS_SIGNAL_FLAGS_NONE,
                                                                   service_changed, self, NULL);
    /* The provider loads before the first context menu: warm the cache now. */
    refresh_devices (self, TRUE);
}

void
nautilus_module_initialize (GTypeModule *module)
{
    nodalix_localsend_provider_register_type (module);
}

void
nautilus_module_shutdown (void)
{
}

void
nautilus_module_list_types (const GType **types,
                            int          *num_types)
{
    static GType type_list[1] = { 0 };

    type_list[0] = NODALIX_TYPE_LOCALSEND_PROVIDER;
    *types = type_list;
    *num_types = G_N_ELEMENTS (type_list);
}
