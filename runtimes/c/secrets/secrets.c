// Secret settings in the system's protected store (DESIGN 5.27); see secrets.h.

#include "secrets/secrets.h"

#include <string.h>

#include "clarity/core.h"
#include "clarity/settings.h"

void clarity_secrets_use(void) {
    clarity_secret_store = (ClaritySecretStore){clarity_secret_get, clarity_secret_set};
}

#if defined(_WIN32)

#include <windows.h>
#include <wincred.h>

static const char* target_of(const char* program, const char* name) {
    return clarity_format("%s:%s", program, name);
}

const char* clarity_secret_get(const char* program, const char* name) {
    PCREDENTIALA found = NULL;
    if (!CredReadA(target_of(program, name), CRED_TYPE_GENERIC, 0, &found)) return NULL;
    const char* secret = clarity_copy_n((const char*)found->CredentialBlob, found->CredentialBlobSize);
    CredFree(found);
    return secret;
}

bool clarity_secret_set(const char* program, const char* name, const char* value) {
    const char* target = target_of(program, name);
    if (!value || !*value) {
        if (CredDeleteA(target, CRED_TYPE_GENERIC, 0) || GetLastError() == ERROR_NOT_FOUND) return true;
        return clarity_fail(CLARITY_STORAGE, "the Credential Manager refused to forget the secret '%s'", name);
    }
    CREDENTIALA credential = {0};
    credential.Type = CRED_TYPE_GENERIC;
    credential.TargetName = (LPSTR)target;
    credential.UserName = (LPSTR)name;
    credential.CredentialBlob = (LPBYTE)value;
    credential.CredentialBlobSize = (DWORD)strlen(value);
    credential.Persist = CRED_PERSIST_LOCAL_MACHINE;
    return CredWriteA(&credential, 0) || clarity_fail(CLARITY_STORAGE, "the Credential Manager refused to keep the secret '%s'", name);
}

#elif defined(__APPLE__)

#include <Security/Security.h>

const char* clarity_secret_get(const char* program, const char* name) {
    UInt32 length = 0;
    void* data = NULL;
    if (SecKeychainFindGenericPassword(NULL, (UInt32)strlen(program), program, (UInt32)strlen(name), name, &length, &data, NULL) != errSecSuccess) return NULL;
    const char* secret = clarity_copy_n((const char*)data, length);
    SecKeychainItemFreeContent(NULL, data);
    return secret;
}

bool clarity_secret_set(const char* program, const char* name, const char* value) {
    SecKeychainItemRef item = NULL;
    OSStatus status = SecKeychainFindGenericPassword(NULL, (UInt32)strlen(program), program, (UInt32)strlen(name), name, NULL, NULL, &item);
    bool forget = !value || !*value;
    if (status == errSecSuccess) {
        status = forget ? SecKeychainItemDelete(item) : SecKeychainItemModifyAttributesAndData(item, NULL, (UInt32)strlen(value), value);
        CFRelease(item);
    } else if (status == errSecItemNotFound) {
        status = forget ? errSecSuccess : SecKeychainAddGenericPassword(NULL, (UInt32)strlen(program), program, (UInt32)strlen(name), name, (UInt32)strlen(value), value, NULL);
    }
    return status == errSecSuccess || clarity_fail(CLARITY_STORAGE, "the Keychain refused the secret '%s' (%d)", name, (int)status);
}

#else

#include <gio/gio.h>

#define SECRETS "org.freedesktop.secrets"
#define SERVICE_PATH "/org/freedesktop/secrets"

static GDBusConnection* session_bus(void) {
    static GDBusConnection* bus = NULL;
    if (!bus) bus = g_bus_get_sync(G_BUS_TYPE_SESSION, NULL, NULL);
    return bus;
}

static GVariant* call(const char* path, const char* interface, const char* method, GVariant* args, const char* reply, GError** error) {
    GDBusConnection* bus = session_bus();
    if (!bus) {
        if (args) g_variant_unref(g_variant_ref_sink(args));
        return NULL;
    }
    return g_dbus_connection_call_sync(bus, SECRETS, path, interface, method, args, G_VARIANT_TYPE(reply), G_DBUS_CALL_FLAGS_NONE, -1, NULL, error);
}

static GVariant* attributes(const char* program, const char* name) {
    GVariantBuilder builder;
    g_variant_builder_init(&builder, G_VARIANT_TYPE("a{ss}"));
    g_variant_builder_add(&builder, "{ss}", "service", program);
    g_variant_builder_add(&builder, "{ss}", "username", name);
    return g_variant_builder_end(&builder);
}

/// A plain session for the secrets: they travel unencrypted on the session bus, as libsecret's do by default.
static char* open_session(void) {
    GVariant* reply = call(SERVICE_PATH, "org.freedesktop.Secret.Service", "OpenSession", g_variant_new("(sv)", "plain", g_variant_new_string("")), "(vo)", NULL);
    if (!reply) return NULL;
    GVariant* output = NULL;
    char* session = NULL;
    g_variant_get(reply, "(vo)", &output, &session);
    g_variant_unref(output);
    g_variant_unref(reply);
    return session;
}

/// The unlocked items of the entry, as a strv the caller frees; a locked keyring holds none it can read.
static char** items_of(const char* program, const char* name) {
    GVariant* reply = call(SERVICE_PATH, "org.freedesktop.Secret.Service", "SearchItems", g_variant_new("(@a{ss})", attributes(program, name)), "(aoao)", NULL);
    if (!reply) return NULL;
    char** unlocked = NULL;
    char** locked = NULL;
    g_variant_get(reply, "(^ao^ao)", &unlocked, &locked);
    g_strfreev(locked);
    g_variant_unref(reply);
    return unlocked;
}

const char* clarity_secret_get(const char* program, const char* name) {
    char** items = items_of(program, name);
    if (!items || !items[0]) {
        g_strfreev(items);
        return NULL;
    }
    char* session = open_session();
    const char* secret = NULL;
    if (session) {
        GVariant* reply = call(items[0], "org.freedesktop.Secret.Item", "GetSecret", g_variant_new("(o)", session), "((oayays))", NULL);
        if (reply) {
            GVariant* value = NULL;
            g_variant_get(reply, "((o@ay@ays))", NULL, NULL, &value, NULL);
            gsize length = 0;
            const char* bytes = g_variant_get_fixed_array(value, &length, 1);
            secret = clarity_copy_n(bytes ? bytes : "", length);
            g_variant_unref(value);
            g_variant_unref(reply);
        }
        g_free(session);
    }
    g_strfreev(items);
    return secret;
}

bool clarity_secret_set(const char* program, const char* name, const char* value) {
    if (!value || !*value) {
        char** items = items_of(program, name);
        for (char** item = items; item && *item; ++item) {
            GVariant* reply = call(*item, "org.freedesktop.Secret.Item", "Delete", NULL, "(o)", NULL);
            if (reply) g_variant_unref(reply);
        }
        g_strfreev(items);
        return true;
    }
    char* session = open_session();
    if (!session) return clarity_fail(CLARITY_STORAGE, "the Secret Service cannot be reached to keep the secret '%s'", name);
    GVariantBuilder properties;
    g_variant_builder_init(&properties, G_VARIANT_TYPE("a{sv}"));
    g_variant_builder_add(&properties, "{sv}", "org.freedesktop.Secret.Item.Label", g_variant_new_take_string(g_strdup_printf("%s: %s", program, name)));
    g_variant_builder_add(&properties, "{sv}", "org.freedesktop.Secret.Item.Attributes", attributes(program, name));
    GVariant* secret = g_variant_new("(o@ay@ays)", session, g_variant_new_fixed_array(G_VARIANT_TYPE_BYTE, "", 0, 1),
                                     g_variant_new_fixed_array(G_VARIANT_TYPE_BYTE, value, strlen(value), 1), "text/plain");
    GError* error = NULL;
    GVariant* reply = call("/org/freedesktop/secrets/aliases/default", "org.freedesktop.Secret.Collection", "CreateItem",
                           g_variant_new("(@a{sv}@(oayays)b)", g_variant_builder_end(&properties), secret, TRUE), "(oo)", &error);
    g_free(session);
    if (!reply) {
        bool failed = clarity_fail(CLARITY_STORAGE, "the Secret Service refused the secret '%s': %s", name, error ? error->message : "no answer");
        g_clear_error(&error);
        return failed;
    }
    char* prompt = NULL;
    g_variant_get(reply, "(oo)", NULL, &prompt);
    bool locked = strcmp(prompt, "/") != 0;
    g_free(prompt);
    g_variant_unref(reply);
    return !locked || clarity_fail(CLARITY_STORAGE, "the keyring is locked; unlock it and keep the secret '%s' again", name);
}

#endif
