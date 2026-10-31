// Secret settings (DESIGN 5.27): kept in the system's protected store, per
// person and per application, never in settings.json. Windows keeps them in
// the Credential Manager, macOS in the Keychain, other systems in the Secret
// Service over D-Bus (GNOME Keyring, KWallet). An entry is the program's name
// with the setting's, as the other stacks keep it; on Windows the target
// `program:name` and the value's UTF-8 bytes (conformance/secret-store.json).

using System.Runtime.InteropServices;
using System.Text;
using Tmds.DBus.Protocol;

namespace Clarity.Runtime;

public static class Secrets
{
    /// <summary>
    /// Where secrets are kept instead of the system's protected store: a host
    /// that has none, as a test that runs the app apart from the person's
    /// desktop (and its keyring). None: the system's store.
    /// </summary>
    public static (Func<string, string, string?> Get, Action<string, string, string> Set)? Store { get; set; }

    /// <summary>The secret <paramref name="name"/> of <paramref name="program"/>; null when it was never kept or the store cannot be reached.</summary>
    public static string? Get(string program, string name)
    {
        if (Store is { } store)
            return store.Get(program, name);
        try
        {
            if (OperatingSystem.IsWindows())
                return WindowsCredentials.Read(WindowsTarget(program, name));
            if (OperatingSystem.IsMacOS())
                return MacKeychain.Find(program, name);
            return Task.Run(() => SecretService.Get(program, name)).GetAwaiter().GetResult();
        }
        catch (Exception e) when (e is DBusExceptionBase or IOException or InvalidOperationException or ExternalException)
        {
            return null;
        }
    }

    /// <summary>Keeps <paramref name="value"/> as the secret; empty text forgets it.</summary>
    public static void Set(string program, string name, string value)
    {
        if (Store is { } store)
        {
            store.Set(program, name, value);
            return;
        }
        try
        {
            if (OperatingSystem.IsWindows())
                WindowsCredentials.Write(WindowsTarget(program, name), name, value);
            else if (OperatingSystem.IsMacOS())
                MacKeychain.Keep(program, name, value);
            else
                Task.Run(() => SecretService.Set(program, name, value)).GetAwaiter().GetResult();
        }
        catch (Exception e) when (e is DBusExceptionBase or IOException or InvalidOperationException or ExternalException)
        {
            throw new ValueException($"cannot keep the secret '{name}' in the system's protected store: {e.Message}");
        }
    }

    /// <summary>The Credential Manager's target of the secret <paramref name="name"/> of <paramref name="program"/>.</summary>
    public static string WindowsTarget(string program, string name) => $"{program}:{name}";

    /// <summary>What the Credential Manager keeps of a secret: its UTF-8 bytes.</summary>
    public static byte[] WindowsBlob(string value) => Encoding.UTF8.GetBytes(value);

    /// <summary>The secret a Credential Manager blob holds.</summary>
    public static string FromWindowsBlob(byte[] blob) => Encoding.UTF8.GetString(blob);

    /// <summary>The Secret Service of the desktop session (org.freedesktop.secrets).</summary>
    static class SecretService
    {
        const string Bus = "org.freedesktop.secrets";
        const string ServicePath = "/org/freedesktop/secrets";
        const string Default = "/org/freedesktop/secrets/aliases/default";

        static DBusConnection Connection => DBusConnection.Session ?? throw new InvalidOperationException("there is no D-Bus session for the Secret Service");

        // Each message is written whole before it is sent: a writer does not live across an await.
        static MessageBuffer OpenSessionMessage()
        {
            using var writer = Connection.GetMessageWriter();
            writer.WriteMethodCallHeader(Bus, ServicePath, "org.freedesktop.Secret.Service", "OpenSession", "sv");
            writer.WriteString("plain");
            writer.WriteVariantString("");
            return writer.CreateMessage();
        }

        static MessageBuffer SearchMessage(string program, string name)
        {
            using var writer = Connection.GetMessageWriter();
            writer.WriteMethodCallHeader(Bus, ServicePath, "org.freedesktop.Secret.Service", "SearchItems", "a{ss}");
            var attributes = writer.WriteDictionaryStart();
            foreach (var (key, value) in new[] { ("service", program), ("username", name) })
            {
                writer.WriteDictionaryEntryStart();
                writer.WriteString(key);
                writer.WriteString(value);
            }
            writer.WriteDictionaryEnd(attributes);
            return writer.CreateMessage();
        }

        static MessageBuffer GetSecretMessage(ObjectPath item, ObjectPath session)
        {
            using var writer = Connection.GetMessageWriter();
            writer.WriteMethodCallHeader(Bus, item.ToString(), "org.freedesktop.Secret.Item", "GetSecret", "o");
            writer.WriteObjectPath(session);
            return writer.CreateMessage();
        }

        static MessageBuffer DeleteMessage(ObjectPath item)
        {
            using var writer = Connection.GetMessageWriter();
            writer.WriteMethodCallHeader(Bus, item.ToString(), "org.freedesktop.Secret.Item", "Delete", null);
            return writer.CreateMessage();
        }

        static MessageBuffer CreateMessage(string program, string name, string value, ObjectPath session)
        {
            using var writer = Connection.GetMessageWriter();
            writer.WriteMethodCallHeader(Bus, Default, "org.freedesktop.Secret.Collection", "CreateItem", "a{sv}(oayays)b");
            var properties = writer.WriteDictionaryStart();
            writer.WriteDictionaryEntryStart();
            writer.WriteString("org.freedesktop.Secret.Item.Label");
            writer.WriteVariantString($"{program}: {name}");
            writer.WriteDictionaryEntryStart();
            writer.WriteString("org.freedesktop.Secret.Item.Attributes");
            writer.WriteVariant(new Dict<string, string> { { "service", program }, { "username", name } }.AsVariantValue());
            writer.WriteDictionaryEnd(properties);
            writer.WriteStructureStart();
            writer.WriteObjectPath(session);
            writer.WriteArray(Array.Empty<byte>());
            writer.WriteArray(Encoding.UTF8.GetBytes(value));
            writer.WriteString("text/plain");
            writer.WriteBool(true);
            return writer.CreateMessage();
        }

        static Task<ObjectPath> OpenSession() =>
            Connection.CallMethodAsync(OpenSessionMessage(), (Message m, object? _) =>
            {
                var reader = m.GetBodyReader();
                reader.ReadVariantValue();
                return reader.ReadObjectPath();
            }, null);

        /// <summary>The unlocked items of the entry; a locked keyring holds none it can read.</summary>
        static Task<ObjectPath[]> Items(string program, string name) =>
            Connection.CallMethodAsync(SearchMessage(program, name), (Message m, object? _) => m.GetBodyReader().ReadArrayOfObjectPath(), null);

        public static async Task<string?> Get(string program, string name)
        {
            var items = await Items(program, name);
            if (items.Length == 0)
                return null;
            var session = await OpenSession();
            return await Connection.CallMethodAsync(GetSecretMessage(items[0], session), (Message m, object? _) =>
            {
                var reader = m.GetBodyReader();
                reader.AlignStruct();
                reader.ReadObjectPath();
                reader.ReadArrayOfByte();
                return Encoding.UTF8.GetString(reader.ReadArrayOfByte());
            }, null);
        }

        public static async Task Set(string program, string name, string value)
        {
            if (value.Length == 0)
            {
                foreach (var item in await Items(program, name))
                    await Connection.CallMethodAsync(DeleteMessage(item));
                return;
            }
            var session = await OpenSession();
            var prompt = await Connection.CallMethodAsync(CreateMessage(program, name, value, session), (Message m, object? _) =>
            {
                var reader = m.GetBodyReader();
                reader.ReadObjectPath();
                return reader.ReadObjectPath();
            }, null);
            if (prompt.ToString() != "/")
                throw new InvalidOperationException("the keyring is locked; unlock it and keep the secret again");
        }
    }

    /// <summary>The Credential Manager of Windows: a generic credential per entry.</summary>
    static class WindowsCredentials
    {
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        struct Credential
        {
            public int Flags;
            public int Type;
            public string TargetName;
            public string? Comment;
            public long LastWritten;
            public int CredentialBlobSize;
            public IntPtr CredentialBlob;
            public int Persist;
            public int AttributeCount;
            public IntPtr Attributes;
            public string? TargetAlias;
            public string UserName;
        }

        const int Generic = 1;
        const int LocalMachine = 2;
        const int NotFound = 1168;

        [DllImport("advapi32.dll", EntryPoint = "CredReadW", CharSet = CharSet.Unicode, SetLastError = true)]
        static extern bool CredRead(string target, int type, int flags, out IntPtr credential);

        [DllImport("advapi32.dll", EntryPoint = "CredWriteW", CharSet = CharSet.Unicode, SetLastError = true)]
        static extern bool CredWrite(ref Credential credential, int flags);

        [DllImport("advapi32.dll", EntryPoint = "CredDeleteW", CharSet = CharSet.Unicode, SetLastError = true)]
        static extern bool CredDelete(string target, int type, int flags);

        [DllImport("advapi32.dll")]
        static extern void CredFree(IntPtr buffer);

        public static string? Read(string target)
        {
            if (!CredRead(target, Generic, 0, out var found))
                return null;
            try
            {
                var credential = Marshal.PtrToStructure<Credential>(found);
                var blob = new byte[credential.CredentialBlobSize];
                Marshal.Copy(credential.CredentialBlob, blob, 0, blob.Length);
                var value = FromWindowsBlob(blob);
                Array.Clear(blob);
                return value;
            }
            finally
            {
                CredFree(found);
            }
        }

        public static void Write(string target, string user, string value)
        {
            if (value.Length == 0)
            {
                if (!CredDelete(target, Generic, 0) && Marshal.GetLastWin32Error() != NotFound)
                    throw new ExternalException("the Credential Manager refused to forget it", Marshal.GetLastWin32Error());
                return;
            }
            var bytes = WindowsBlob(value);
            var blob = Marshal.AllocCoTaskMem(bytes.Length);
            try
            {
                Marshal.Copy(bytes, 0, blob, bytes.Length);
                var credential = new Credential { Type = Generic, TargetName = target, UserName = user, CredentialBlob = blob, CredentialBlobSize = bytes.Length, Persist = LocalMachine };
                if (!CredWrite(ref credential, 0))
                    throw new ExternalException("the Credential Manager refused to keep it", Marshal.GetLastWin32Error());
            }
            finally
            {
                Marshal.Copy(new byte[bytes.Length], 0, blob, bytes.Length);
                Marshal.FreeCoTaskMem(blob);
                Array.Clear(bytes);
            }
        }
    }

    /// <summary>The Keychain of macOS: a generic password per entry, its service the program and its account the setting.</summary>
    static class MacKeychain
    {
        const string Security = "/System/Library/Frameworks/Security.framework/Security";
        const string CoreFoundation = "/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation";
        const int ItemNotFound = -25300;

        [DllImport(Security)]
        static extern int SecKeychainFindGenericPassword(IntPtr keychain, uint serviceLength, byte[] service, uint accountLength, byte[] account, out uint passwordLength, out IntPtr password, out IntPtr item);

        [DllImport(Security)]
        static extern int SecKeychainAddGenericPassword(IntPtr keychain, uint serviceLength, byte[] service, uint accountLength, byte[] account, uint passwordLength, byte[] password, IntPtr item);

        [DllImport(Security)]
        static extern int SecKeychainItemModifyAttributesAndData(IntPtr item, IntPtr attributes, uint length, byte[] data);

        [DllImport(Security)]
        static extern int SecKeychainItemDelete(IntPtr item);

        [DllImport(Security)]
        static extern int SecKeychainItemFreeContent(IntPtr attributes, IntPtr data);

        [DllImport(CoreFoundation)]
        static extern void CFRelease(IntPtr reference);

        public static string? Find(string program, string name)
        {
            var (service, account) = (Encoding.UTF8.GetBytes(program), Encoding.UTF8.GetBytes(name));
            if (SecKeychainFindGenericPassword(IntPtr.Zero, (uint)service.Length, service, (uint)account.Length, account, out var length, out var data, out var item) != 0)
                return null;
            try
            {
                var bytes = new byte[length];
                Marshal.Copy(data, bytes, 0, (int)length);
                return Encoding.UTF8.GetString(bytes);
            }
            finally
            {
                SecKeychainItemFreeContent(IntPtr.Zero, data);
                CFRelease(item);
            }
        }

        public static void Keep(string program, string name, string value)
        {
            var (service, account, password) = (Encoding.UTF8.GetBytes(program), Encoding.UTF8.GetBytes(name), Encoding.UTF8.GetBytes(value));
            var status = SecKeychainFindGenericPassword(IntPtr.Zero, (uint)service.Length, service, (uint)account.Length, account, out _, out var data, out var item);
            if (status == 0)
            {
                SecKeychainItemFreeContent(IntPtr.Zero, data);
                try
                {
                    status = value.Length == 0 ? SecKeychainItemDelete(item) : SecKeychainItemModifyAttributesAndData(item, IntPtr.Zero, (uint)password.Length, password);
                }
                finally
                {
                    CFRelease(item);
                }
            }
            else if (status == ItemNotFound)
            {
                status = value.Length == 0 ? 0 : SecKeychainAddGenericPassword(IntPtr.Zero, (uint)service.Length, service, (uint)account.Length, account, (uint)password.Length, password, IntPtr.Zero);
            }
            if (status != 0)
                throw new ExternalException($"the Keychain answered {status}", status);
        }
    }
}
