// Secret settings in the system's protected store (DESIGN 5.27); see secrets.hpp.

#include "secrets/secrets.hpp"

#include <cstring>
#include <vector>

#include "clarity/errors.hpp"
#include "clarity/settings.hpp"

namespace clarity {

void secrets_use() {
    secret_store = SecretStore{secret_get, secret_set};
}

}  // namespace clarity

#if defined(_WIN32)

#include <windows.h>
#include <wincred.h>

namespace clarity {

std::optional<std::string> secret_get(const std::string& program, const std::string& name) {
    PCREDENTIALA found = nullptr;
    if (!CredReadA((program + ":" + name).c_str(), CRED_TYPE_GENERIC, 0, &found)) return std::nullopt;
    std::string secret(reinterpret_cast<const char*>(found->CredentialBlob), found->CredentialBlobSize);
    CredFree(found);
    return secret;
}

void secret_set(const std::string& program, const std::string& name, const std::string& value) {
    std::string target = program + ":" + name;
    if (value.empty()) {
        if (!CredDeleteA(target.c_str(), CRED_TYPE_GENERIC, 0) && GetLastError() != ERROR_NOT_FOUND) throw StorageError("the Credential Manager refused to forget the secret '" + name + "'");
        return;
    }
    CREDENTIALA credential{};
    credential.Type = CRED_TYPE_GENERIC;
    credential.TargetName = target.data();
    credential.UserName = const_cast<char*>(name.c_str());
    credential.CredentialBlob = reinterpret_cast<LPBYTE>(const_cast<char*>(value.data()));
    credential.CredentialBlobSize = static_cast<DWORD>(value.size());
    credential.Persist = CRED_PERSIST_LOCAL_MACHINE;
    if (!CredWriteA(&credential, 0)) throw StorageError("the Credential Manager refused to keep the secret '" + name + "'");
}

}  // namespace clarity

#elif defined(__APPLE__)

#include <Security/Security.h>

namespace clarity {

std::optional<std::string> secret_get(const std::string& program, const std::string& name) {
    UInt32 length = 0;
    void* data = nullptr;
    if (SecKeychainFindGenericPassword(nullptr, static_cast<UInt32>(program.size()), program.c_str(), static_cast<UInt32>(name.size()), name.c_str(), &length, &data, nullptr) != errSecSuccess) return std::nullopt;
    std::string secret(static_cast<const char*>(data), length);
    SecKeychainItemFreeContent(nullptr, data);
    return secret;
}

void secret_set(const std::string& program, const std::string& name, const std::string& value) {
    SecKeychainItemRef item = nullptr;
    OSStatus status = SecKeychainFindGenericPassword(nullptr, static_cast<UInt32>(program.size()), program.c_str(), static_cast<UInt32>(name.size()), name.c_str(), nullptr, nullptr, &item);
    if (status == errSecSuccess) {
        status = value.empty() ? SecKeychainItemDelete(item) : SecKeychainItemModifyAttributesAndData(item, nullptr, static_cast<UInt32>(value.size()), value.data());
        CFRelease(item);
    } else if (status == errSecItemNotFound) {
        status = value.empty() ? errSecSuccess
                               : SecKeychainAddGenericPassword(nullptr, static_cast<UInt32>(program.size()), program.c_str(), static_cast<UInt32>(name.size()), name.c_str(),
                                                               static_cast<UInt32>(value.size()), value.data(), nullptr);
    }
    if (status != errSecSuccess) throw StorageError("the Keychain refused the secret '" + name + "' (" + std::to_string(status) + ")");
}

}  // namespace clarity

#else

#include <gio/gio.h>

namespace clarity {

namespace {

const char* const Secrets = "org.freedesktop.secrets";
const char* const ServicePath = "/org/freedesktop/secrets";

GDBusConnection* session_bus() {
    static GDBusConnection* bus = g_bus_get_sync(G_BUS_TYPE_SESSION, nullptr, nullptr);
    return bus;
}

GVariant* call(const std::string& path, const char* interface, const char* method, GVariant* args, const char* reply, GError** error = nullptr) {
    GDBusConnection* bus = session_bus();
    if (!bus) {
        if (args) g_variant_unref(g_variant_ref_sink(args));
        return nullptr;
    }
    return g_dbus_connection_call_sync(bus, Secrets, path.c_str(), interface, method, args, G_VARIANT_TYPE(reply), G_DBUS_CALL_FLAGS_NONE, -1, nullptr, error);
}

GVariant* attributes(const std::string& program, const std::string& name) {
    GVariantBuilder builder;
    g_variant_builder_init(&builder, G_VARIANT_TYPE("a{ss}"));
    g_variant_builder_add(&builder, "{ss}", "service", program.c_str());
    g_variant_builder_add(&builder, "{ss}", "username", name.c_str());
    return g_variant_builder_end(&builder);
}

/// A plain session for the secrets: they travel unencrypted on the session bus, as libsecret's do by default.
std::optional<std::string> open_session() {
    GVariant* reply = call(ServicePath, "org.freedesktop.Secret.Service", "OpenSession", g_variant_new("(sv)", "plain", g_variant_new_string("")), "(vo)");
    if (!reply) return std::nullopt;
    GVariant* output = nullptr;
    char* session = nullptr;
    g_variant_get(reply, "(vo)", &output, &session);
    std::string path = session;
    g_free(session);
    g_variant_unref(output);
    g_variant_unref(reply);
    return path;
}

/// The unlocked items of the entry; a locked keyring holds none it can read.
std::vector<std::string> items_of(const std::string& program, const std::string& name) {
    std::vector<std::string> items;
    GVariant* reply = call(ServicePath, "org.freedesktop.Secret.Service", "SearchItems", g_variant_new("(@a{ss})", attributes(program, name)), "(aoao)");
    if (!reply) return items;
    char** unlocked = nullptr;
    char** locked = nullptr;
    g_variant_get(reply, "(^ao^ao)", &unlocked, &locked);
    for (char** item = unlocked; item && *item; ++item) items.emplace_back(*item);
    g_strfreev(unlocked);
    g_strfreev(locked);
    g_variant_unref(reply);
    return items;
}

}  // namespace

std::optional<std::string> secret_get(const std::string& program, const std::string& name) {
    auto items = items_of(program, name);
    if (items.empty()) return std::nullopt;
    auto session = open_session();
    if (!session) return std::nullopt;
    GVariant* reply = call(items.front(), "org.freedesktop.Secret.Item", "GetSecret", g_variant_new("(o)", session->c_str()), "((oayays))");
    if (!reply) return std::nullopt;
    GVariant* value = nullptr;
    g_variant_get(reply, "((o@ay@ays))", nullptr, nullptr, &value, nullptr);
    gsize length = 0;
    const char* bytes = static_cast<const char*>(g_variant_get_fixed_array(value, &length, 1));
    std::string secret(bytes ? bytes : "", length);
    g_variant_unref(value);
    g_variant_unref(reply);
    return secret;
}

void secret_set(const std::string& program, const std::string& name, const std::string& value) {
    if (value.empty()) {
        for (const auto& item : items_of(program, name)) {
            if (GVariant* reply = call(item, "org.freedesktop.Secret.Item", "Delete", nullptr, "(o)")) g_variant_unref(reply);
        }
        return;
    }
    auto session = open_session();
    if (!session) throw StorageError("the Secret Service cannot be reached to keep the secret '" + name + "'");
    GVariantBuilder properties;
    g_variant_builder_init(&properties, G_VARIANT_TYPE("a{sv}"));
    g_variant_builder_add(&properties, "{sv}", "org.freedesktop.Secret.Item.Label", g_variant_new_string((program + ": " + name).c_str()));
    g_variant_builder_add(&properties, "{sv}", "org.freedesktop.Secret.Item.Attributes", attributes(program, name));
    GVariant* secret = g_variant_new("(o@ay@ays)", session->c_str(), g_variant_new_fixed_array(G_VARIANT_TYPE_BYTE, "", 0, 1),
                                     g_variant_new_fixed_array(G_VARIANT_TYPE_BYTE, value.data(), value.size(), 1), "text/plain");
    GError* error = nullptr;
    GVariant* reply = call("/org/freedesktop/secrets/aliases/default", "org.freedesktop.Secret.Collection", "CreateItem",
                           g_variant_new("(@a{sv}@(oayays)b)", g_variant_builder_end(&properties), secret, TRUE), "(oo)", &error);
    if (!reply) {
        std::string why = error ? error->message : "no answer";
        g_clear_error(&error);
        throw StorageError("the Secret Service refused the secret '" + name + "': " + why);
    }
    char* prompt = nullptr;
    g_variant_get(reply, "(oo)", nullptr, &prompt);
    bool locked = std::strcmp(prompt, "/") != 0;
    g_free(prompt);
    g_variant_unref(reply);
    if (locked) throw StorageError("the keyring is locked; unlock it and keep the secret '" + name + "' again");
}

}  // namespace clarity

#endif
