// The desktop shell of C++ applications: a webview per window, whose
// renderer sends its commands to the service (service.cpp). Custom parts
// draw in native child windows (parts.cpp).

#include "shell/shell.hpp"

#include <cstdio>
#include <cstdlib>

#if defined(__APPLE__)
#include <objc/message.h>
#include <objc/runtime.h>
#endif

#if defined(_WIN32)
#include <windows.h>
#include <shobjidl.h>
#endif

#include "shell/parts.hpp"
#include "webview/webview.h"

#if defined(__linux__)
#include <gdk/gdkx.h>
#endif

namespace clarity::shell {

using nlohmann::json;

namespace {

/// A desktop window with a webview showing the renderer.
class Window final : public Desktop {
public:
    Window(std::string name, Service& service, const Options& options, Size size) : name_(std::move(name)), view_(false, nullptr) {
        view_.set_title(options.title);
        view_.set_size(size.width, size.height, WEBVIEW_HINT_NONE);
        // The renderer calls clarityInvoke(command, args); it answers like the Go shell.
        view_.bind("clarityInvoke", [this, &service](const std::string& request) {
            json args = json::parse(request);
            return service.invoke(args.at(0).get<std::string>(), args.size() > 1 ? args.at(1) : json::object(), this).dump();
        });
        view_.set_html(options.html);
    }

    std::string name() const override { return name_; }
    void set_title(const std::string& title) override { view_.set_title(title); }

    void close() override {
#if defined(__linux__)
        if (auto native = view_.window(); native.ok()) gtk_window_close(GTK_WINDOW(native.value()));
#endif
    }

    uintptr_t part_parent() override {
#if defined(__linux__)
        auto widget = view_.widget();
        if (!widget.ok()) return 0;
        GdkWindow* window = gtk_widget_get_window(GTK_WIDGET(widget.value()));
        if (!window || !GDK_IS_X11_WINDOW(window)) return 0;
        gdk_window_ensure_native(window);
        return static_cast<uintptr_t>(GDK_WINDOW_XID(window));
#elif defined(_WIN32)
        // The window itself: the part's window is a child beside the webview's.
        auto window = view_.window();
        if (!window.ok()) return 0;
        return reinterpret_cast<uintptr_t>(window.value());
#else
        return 0;
#endif
    }

    std::optional<std::string> choose_folder(const std::string& title) override {
#if defined(__linux__)
        auto native = view_.window();
        GtkWindow* parent = native.ok() ? GTK_WINDOW(native.value()) : nullptr;
        GtkWidget* dialog = gtk_file_chooser_dialog_new(title.c_str(), parent, GTK_FILE_CHOOSER_ACTION_SELECT_FOLDER, "_Cancel", GTK_RESPONSE_CANCEL, "_Select", GTK_RESPONSE_ACCEPT, nullptr);
        std::optional<std::string> chosen;
        if (gtk_dialog_run(GTK_DIALOG(dialog)) == GTK_RESPONSE_ACCEPT) {
            if (char* folder = gtk_file_chooser_get_filename(GTK_FILE_CHOOSER(dialog))) {
                chosen = folder;
                g_free(folder);
            }
        }
        gtk_widget_destroy(dialog);
        return chosen;
#elif defined(_WIN32)
        IFileOpenDialog* dialog = nullptr;
        if (FAILED(CoCreateInstance(CLSID_FileOpenDialog, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&dialog)))) return std::nullopt;
        FILEOPENDIALOGOPTIONS options = 0;
        dialog->GetOptions(&options);
        dialog->SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM);
        std::wstring wide(MultiByteToWideChar(CP_UTF8, 0, title.c_str(), -1, nullptr, 0), L'\0');
        MultiByteToWideChar(CP_UTF8, 0, title.c_str(), -1, wide.data(), static_cast<int>(wide.size()));
        dialog->SetTitle(wide.c_str());
        std::optional<std::string> chosen;
        auto window = view_.window();
        if (SUCCEEDED(dialog->Show(window.ok() ? static_cast<HWND>(window.value()) : nullptr))) {
            IShellItem* item = nullptr;
            PWSTR path = nullptr;
            if (SUCCEEDED(dialog->GetResult(&item)) && SUCCEEDED(item->GetDisplayName(SIGDN_FILESYSPATH, &path))) {
                std::string utf8(WideCharToMultiByte(CP_UTF8, 0, path, -1, nullptr, 0, nullptr, nullptr), '\0');
                WideCharToMultiByte(CP_UTF8, 0, path, -1, utf8.data(), static_cast<int>(utf8.size()), nullptr, nullptr);
                utf8.resize(utf8.empty() ? 0 : utf8.size() - 1);
                chosen = utf8;
                CoTaskMemFree(path);
            }
            if (item) item->Release();
        }
        dialog->Release();
        return chosen;
#elif defined(__APPLE__)
        id panel = reinterpret_cast<id (*)(Class, SEL)>(objc_msgSend)(objc_getClass("NSOpenPanel"), sel_registerName("openPanel"));
        auto flag = reinterpret_cast<void (*)(id, SEL, BOOL)>(objc_msgSend);
        flag(panel, sel_registerName("setCanChooseDirectories:"), YES);
        flag(panel, sel_registerName("setCanChooseFiles:"), NO);
        flag(panel, sel_registerName("setAllowsMultipleSelection:"), NO);
        flag(panel, sel_registerName("setCanCreateDirectories:"), YES);
        id message = reinterpret_cast<id (*)(Class, SEL, const char*)>(objc_msgSend)(objc_getClass("NSString"), sel_registerName("stringWithUTF8String:"), title.c_str());
        reinterpret_cast<void (*)(id, SEL, id)>(objc_msgSend)(panel, sel_registerName("setMessage:"), message);
        // NSModalResponseOK.
        if (reinterpret_cast<long (*)(id, SEL)>(objc_msgSend)(panel, sel_registerName("runModal")) != 1) return std::nullopt;
        id url = reinterpret_cast<id (*)(id, SEL)>(objc_msgSend)(panel, sel_registerName("URL"));
        const char* path = reinterpret_cast<const char* (*)(id, SEL)>(objc_msgSend)(url, sel_registerName("fileSystemRepresentation"));
        if (!path) return std::nullopt;
        return std::string(path);
#else
        return Desktop::choose_folder(title);
#endif
    }

    void eval(const std::string& js) { view_.eval(js); }
    webview::webview& view() { return view_; }

private:
    std::string name_;
    webview::webview view_;
};

}  // namespace

// While the databases are not open, the title bar's close of the main window
// goes to the screen, which connects again (DESIGN 5.27): each system's own
// close is held back and the screen is asked instead.
namespace {

const char* const close_requested = "window.clarityCloseRequested && window.clarityCloseRequested()";
std::function<bool()> close_to_screen;

#if defined(_WIN32)
WNDPROC window_proc;
LRESULT CALLBACK on_message(HWND hwnd, UINT message, WPARAM wp, LPARAM lp) {
    if (message == WM_CLOSE && close_to_screen && close_to_screen()) return 0;
    return CallWindowProcW(window_proc, hwnd, message, wp, lp);
}
#elif defined(__APPLE__)
id main_ns_window;
BOOL should_close(id, SEL, id sender) { return sender == main_ns_window && close_to_screen && close_to_screen() ? NO : YES; }
#endif

void hold_back_close(Window& main) {
    auto native = main.view().window();
    if (!native.ok()) return;
#if defined(__linux__)
    g_signal_connect(G_OBJECT(native.value()), "delete-event",
                     G_CALLBACK(+[](GtkWidget*, GdkEvent*, gpointer) -> gboolean { return close_to_screen && close_to_screen() ? TRUE : FALSE; }), nullptr);
#elif defined(_WIN32)
    auto hwnd = static_cast<HWND>(native.value());
    window_proc = reinterpret_cast<WNDPROC>(SetWindowLongPtrW(hwnd, GWLP_WNDPROC, reinterpret_cast<LONG_PTR>(on_message)));
#elif defined(__APPLE__)
    main_ns_window = static_cast<id>(native.value());
    id delegate = reinterpret_cast<id (*)(id, SEL)>(objc_msgSend)(main_ns_window, sel_registerName("delegate"));
    if (delegate) class_addMethod(object_getClass(delegate), sel_registerName("windowShouldClose:"), reinterpret_cast<IMP>(should_close), "c@:@");
#endif
}

}  // namespace

int run(const Options& options) {
#if defined(__linux__)
    // WebKitGTK 2.52 paints parts of a new window black through its DMABUF
    // renderer; it must be off before the web process starts.
    setenv("WEBKIT_DISABLE_DMABUF_RENDERER", "1", 0);
    // Custom parts draw in X11 child windows: under Wayland the toolkit runs on XWayland.
    if (parts::present()) setenv("GDK_BACKEND", "x11", 0);
#endif
    Service service(options);
    service.parts = {parts::place, parts::set, parts::hear, parts::close};
    std::map<std::string, std::unique_ptr<Window>> windows;
    windows["main"] = std::make_unique<Window>("main", service, options, window_size(options.size));
    service.quit = [&] { windows["main"]->view().terminate(); };
    close_to_screen = [&] {
        if (!service.closes_to_screen()) return false;
        windows["main"]->eval(close_requested);
        return true;
    };
    hold_back_close(*windows["main"]);
    service.open_desktop = [&](Desktop&, const std::string& name, Size size) {
        auto& window = windows[name] = std::make_unique<Window>(name, service, options, size);
#if defined(__linux__)
        // When it closes, its opener is told, so it can reload its data.
        struct Closing {
            Service* service;
            std::map<std::string, std::unique_ptr<Window>>* windows;
            std::string name;
        };
        if (auto native = window->view().window(); native.ok()) {
            g_signal_connect_data(
                G_OBJECT(native.value()), "destroy",
                G_CALLBACK(+[](GtkWidget*, gpointer data) {
                    auto* c = static_cast<Closing*>(data);
                    if (auto opener = c->service->closed(c->name); opener && c->windows->contains(*opener))
                        c->windows->at(*opener)->eval("window.dispatchEvent(new Event('clarity-return'))");
                }),
                new Closing{&service, &windows, name}, +[](gpointer data, GClosure*) { delete static_cast<Closing*>(data); }, GConnectFlags(0));
        }
#endif
    };
    // A part tells on its own thread: the renderer of its window hears it on the window's.
    if (parts::present()) {
        std::string problem = parts::start([&](uint64_t window, const std::string& part, const std::string& tell, const std::string& values) {
            std::string js = "window.clarityTold && window.clarityTold(" + std::to_string(window) + ", " + json(part).dump() + ", " + json(tell).dump() + ", " + values + ")";
            windows["main"]->view().dispatch([&, window, js] {
                auto name = service.parts_window(window);
                if (name && windows.contains(*name)) windows.at(*name)->eval(js);
            });
        });
        if (!problem.empty()) fprintf(stderr, "%s\n", problem.c_str());
    }
    windows["main"]->view().run();
    close_to_screen = nullptr;
    parts::stop();
    return 0;
}

}  // namespace clarity::shell
