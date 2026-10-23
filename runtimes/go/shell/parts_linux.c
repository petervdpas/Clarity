// The X11 window of the webview that custom parts draw in (DESIGN 6.5).
// Linux only, through cgo.

#include "parts_linux.h"

#include <gdk/gdkx.h>
#include <gtk/gtk.h>
#include <string.h>

/// The WebKitWebView among a widget and its children.
static GtkWidget* web_view_in(GtkWidget* widget) {
    if (strcmp(G_OBJECT_TYPE_NAME(widget), "WebKitWebView") == 0) return widget;
    if (!GTK_IS_CONTAINER(widget)) return NULL;
    GList* children = gtk_container_get_children(GTK_CONTAINER(widget));
    GtkWidget* found = NULL;
    for (GList* c = children; c && !found; c = c->next) found = web_view_in(GTK_WIDGET(c->data));
    g_list_free(children);
    return found;
}

uintptr_t clarity_go_parts_parent(void* window) {
    if (!window) return 0;
    GtkWidget* view = web_view_in(GTK_WIDGET(window));
    GdkWindow* native = view ? gtk_widget_get_window(view) : gtk_widget_get_window(GTK_WIDGET(window));
    if (!native || !GDK_IS_X11_WINDOW(native)) return 0;
    gdk_window_ensure_native(native);
    return (uintptr_t)GDK_WINDOW_XID(native);
}
