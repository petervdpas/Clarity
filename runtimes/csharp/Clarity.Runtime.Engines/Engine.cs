// Engines (DESIGN 5.11): processing modules compiled once to WebAssembly,
// which every stack hosts alike (conformance/engines.json). An engine
// computes; it imports only the services of Clarity it declares (`uses
// clock, random, log`), so it reaches no database, file or network.
// Each call runs on a fresh instance of the module, so an engine keeps no
// state from one call to the next, unless it is declared with `state per
// app` (one instance serves every call) or `state per window` (each open
// window has an instance of its own, dropped when the window closes). Every
// call runs within the limits of its engine:
// a call that runs too long is stopped, and an engine never gets more memory
// than its limit. Values cross as Clarity's JSON:
// decimals as exact text, enums by name, records by their field names.

using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace Clarity.Runtime;

/// What one call of an engine may take: how long it may run, and how much
/// memory the engine may grow to.
public readonly record struct EngineLimits(uint Seconds, uint Megabytes);

/// What the services give: the system's clock, a fresh seed for every call
/// and the standard error, unless a test fixes the clock (milliseconds since
/// 1970) or the seed, or keeps the lines of the log.
/// Whether an engine keeps an instance from call to call.
public enum EngineState
{
    /// Every call runs on a fresh instance.
    None,

    /// One instance serves every call of the application (`state per app`).
    PerApp,

    /// Each open window has an instance of its own (`state per window`).
    PerWindow,
}

/// An engine as its declaration gives it: its functions, the services it
/// uses, the limits of a call, whether it keeps state and whether it has
/// `on open` (the engine's `clarity_open`, run on every new instance).
public sealed record EngineDeclaration(
    IReadOnlyList<string> Functions,
    IReadOnlyList<string> Uses,
    EngineLimits Limits,
    EngineState State = EngineState.None,
    bool Opens = false,
    bool Screen = false);

/// <summary>
/// What the services give in place of the system's: a fixed clock or seed,
/// where the log goes, and the language <c>language</c> gives (none for
/// <see cref="Engine.Language"/>, the one the screen picked).
/// </summary>
public sealed record EngineServices(long? Clock = null, ulong? Seed = null, Action<string>? Log = null, Func<string?>? Language = null);

public sealed class Engine
{
    readonly string name;
    readonly Wasmtime.Engine engine;
    readonly Wasmtime.Module module;
    readonly EngineDeclaration declared;
    readonly EngineServices services;
    readonly object gate = new();
    /// The instances an engine with state keeps between calls, by window
    /// (one, at 0, for state per app).
    readonly Dictionary<long, Live> kept = [];

    IReadOnlyList<string> uses => declared.Uses;
    EngineLimits limits => declared.Limits;

    Engine(string name, Wasmtime.Engine engine, Wasmtime.Module module, EngineDeclaration declared, EngineServices services)
    {
        this.name = name;
        this.engine = engine;
        this.module = module;
        this.declared = declared;
        this.services = services;
        // A window that closes takes its instance along.
        if (declared.State == EngineState.PerWindow)
            OpenWindows.Closed += CloseWindow;
    }

    /// The services an engine may use, each a function of module `clarity`
    /// with its parameters and results.
    static readonly Dictionary<string, (Wasmtime.ValueKind[] Parameters, Wasmtime.ValueKind[] Results)> Services = new()
    {
        ["clock"] = ([], [Wasmtime.ValueKind.Int64]),
        ["random"] = ([], [Wasmtime.ValueKind.Int64]),
        ["log"] = ([Wasmtime.ValueKind.Int32, Wasmtime.ValueKind.Int32], []),
        ["language"] = ([Wasmtime.ValueKind.Int32, Wasmtime.ValueKind.Int32], [Wasmtime.ValueKind.Int32]),
    };

    /// <summary>The language the screen picked, as its tag (<c>nl</c>); null for the texts as written. The application sets it.</summary>
    public static Func<string?> Language { get; set; } = () => null;

    /// The engine with `services` in place of the system's.
    public Engine WithServices(EngineServices services) => new(name, engine, module, declared, services);

    /// The next number of splitmix64, the random sequence every host gives alike.
    static long Splitmix(ref ulong state)
    {
        unchecked
        {
            state += 0x9E3779B97F4A7C15;
            var z = state;
            z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9;
            z = (z ^ (z >> 27)) * 0x94D049BB133111EB;
            return (long)(z ^ (z >> 31));
        }
    }

    /// Values as Clarity writes them.
    public static readonly JsonSerializerOptions Json = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        Converters = { new DecimalAsText(), new JsonStringEnumConverter() },
    };

    /// Loads `bytes` as engine `name` with the functions it is declared with,
    /// the services it uses and the limits of its calls; refuses a module that
    /// is no engine, imports anything but those services, or lacks one of the functions.
    public static Engine Load(string name, byte[] bytes, EngineDeclaration declaration)
    {
        var (functions, uses) = (declaration.Functions, declaration.Uses);
        EngineException Refused(string why) => new($"engine {name} {why}");
        if (bytes.Length < 4 || bytes[0] != 0 || bytes[1] != 'a' || bytes[2] != 's' || bytes[3] != 'm')
            throw Refused("is no WebAssembly module");
        // A call is interrupted when the engine's epoch moves past its deadline.
        using var config = new Wasmtime.Config().WithEpochInterruption(true);
        var engine = new Wasmtime.Engine(config);
        Wasmtime.Module module;
        try
        {
            module = Wasmtime.Module.FromBytes(engine, name, bytes);
        }
        catch (Exception e)
        {
            throw Refused($"is no WebAssembly module: {e.Message}");
        }
        foreach (var import in module.Imports)
        {
            if (import.ModuleName != "clarity" || !uses.Contains(import.Name))
                throw Refused($"imports {import.ModuleName}.{import.Name}; an engine imports only the services it uses");
            if (import is not Wasmtime.FunctionImport function || !Services.TryGetValue(import.Name, out var type)
                || !function.Parameters.SequenceEqual(type.Parameters) || !function.Results.SequenceEqual(type.Results))
                throw Refused($"imports clarity.{import.Name} with another type than Clarity gives it");
        }
        var exported = module.Exports.Where(e => e is Wasmtime.FunctionExport).Select(e => e.Name).ToHashSet();
        foreach (var needed in new[] { "clarity_alloc", "clarity_free" })
            if (!exported.Contains(needed))
                throw Refused($"lacks {needed} of the engine interface");
        if (!module.Exports.Any(e => e is Wasmtime.MemoryExport && e.Name == "memory"))
            throw Refused("lacks the memory of the engine interface");
        if (functions.FirstOrDefault(f => !exported.Contains(f)) is { } missing)
            throw Refused($"has no function {missing}");
        // An engine beside the screen exports its frame loop (DESIGN 5.12).
        foreach (var needed in declaration.Screen ? new[] { "clarity_tick", "clarity_draw" } : [])
            if (!exported.Contains(needed))
                throw Refused($"lacks {needed}, which an engine beside the screen needs");
        if (declaration.Opens && !exported.Contains("clarity_open"))
            throw Refused("lacks clarity_open, which its on open needs");
        return new Engine(name, engine, module, declaration, new EngineServices());
    }

    /// Calls `function` with `args`: its value as JSON, the engine's refusal
    /// (a RefusedException in its own words), or what went wrong running it.
    public JsonNode? Call(string function, params object?[] args) => CallIn(0, function, args);

    /// Drops the instance window `window` had, when it closes.
    public void CloseWindow(long window)
    {
        if (declared.State != EngineState.PerWindow)
            return;
        lock (gate)
            if (kept.Remove(window, out var live))
                live.Dispose();
    }

    /// Calls `function` in open window `window`: for an engine with state per
    /// window, on that window's own instance.
    public JsonNode? CallIn(long window, string function, params object?[] args)
    {
        EngineException Failed(string why) => new($"engine {name} failed in {function}: {why}");
        // An engine with state keeps its instance, unless a call fails or runs
        // past its limit: then its memory may be half changed, so the next
        // call starts a new one. Its calls take turns.
        var keeps = declared.State != EngineState.None;
        var key = declared.State == EngineState.PerWindow ? window : 0;
        lock (keeps ? gate : new object())
        {
            Live? live = null;
            if (keeps && kept.Remove(key, out var found))
                live = found;
            byte[] output;
            // The epoch moves once the time is up, which stops the call at its
            // next check. The timer moves it only while this call runs, so a
            // late timer never cuts short the next call.
            var call = new Running();
            using (var late = new Timer(_ =>
            {
                lock (call)
                    if (!call.Done)
                        engine.IncrementEpoch();
            }, null, TimeSpan.FromSeconds(limits.Seconds), Timeout.InfiniteTimeSpan))
            using (call.Finish())
            {
                try
                {
                    live ??= Instance();
                    output = Invoke(live, function, args);
                }
                catch (Exception e)
                {
                    live?.Dispose();
                    throw e switch
                    {
                        OpenFailed open => Failed(open.Message),
                        EngineException => e,
                        Wasmtime.TrapException { Type: Wasmtime.TrapCode.Interrupt } => Failed($"it ran longer than {limits.Seconds} second{(limits.Seconds == 1 ? "" : "s")}"),
                        _ => Failed(e.Message),
                    };
                }
            }
            // The engine answered, even with a refusal, so its memory is whole.
            if (keeps)
                kept[key] = live;
            else
                live.Dispose();
            return Answer(output, Failed);
        }
    }

    /// The value in an engine's answer, or its refusal in its own words.
    static JsonNode? Answer(byte[] output, Func<string, EngineException> failed)
    {
        JsonNode? answer;
        try
        {
            answer = JsonNode.Parse(Encoding.UTF8.GetString(output));
        }
        catch (JsonException e)
        {
            throw failed($"its answer is no JSON: {e.Message}");
        }
        if (answer?["error"] is { } error)
            // An engine refuses by the key of its text (its `says`), or in its own words.
            throw new RefusedException(Text.Of(error.GetValue<string>()));
        if (answer is not JsonObject found || !found.ContainsKey("value"))
            throw failed("its answer has no value");
        return found["value"];
    }

    /// An instance with what a call needs of it, and the state of its services.
    sealed class Live(Wasmtime.Store store, Wasmtime.Linker linker, Wasmtime.Instance instance, Serving serving) : IDisposable
    {
        public readonly Wasmtime.Store Store = store;
        public readonly Wasmtime.Instance Instance = instance;
        public readonly Serving Serving = serving;

        public void Dispose()
        {
            linker.Dispose();
            Store.Dispose();
        }
    }

    /// The random sequence of the call an instance runs.
    sealed class Serving
    {
        public ulong Random;
    }

    /// A new instance of the module with its services, opened when the engine has `on open`.
    Live Instance()
    {
        var store = new Wasmtime.Store(engine);
        store.SetLimits(memorySize: limits.Megabytes * 1024L * 1024L);
        var serving = new Serving();
        var linker = Linker(serving);
        var live = new Live(store, linker, linker.Instantiate(store, module), serving);
        if (!declared.Opens)
            return live;
        try
        {
            var output = Invoke(live, "clarity_open", []);
            Answer(output, _ => new EngineException("its on open gave no answer"));
        }
        catch (RefusedException e)
        {
            live.Dispose();
            throw new OpenFailed($"its on open refused: {e.Message}");
        }
        catch (EngineException)
        {
            live.Dispose();
            throw new OpenFailed("its on open gave no answer");
        }
        return live;
    }

    /// Why `on open` failed, which the call it came before reports.
    sealed class OpenFailed(string why) : Exception(why);

    /// Writes the arguments into the instance, runs `function` and reads its
    /// answer. The random sequence starts afresh for every call.
    byte[] Invoke(Live live, string function, object?[] args)
    {
        live.Serving.Random = services.Seed ?? (ulong)Random.Shared.NextInt64(long.MinValue, long.MaxValue);
        live.Store.SetEpochDeadline(1);
        var instance = live.Instance;
        var memory = instance.GetMemory("memory") ?? throw new EngineException("no memory");
        var alloc = instance.GetFunction<int, int>("clarity_alloc") ?? throw new EngineException("no clarity_alloc");
        var free = instance.GetAction<int, int>("clarity_free") ?? throw new EngineException("no clarity_free");
        var run = instance.GetFunction<int, int, long>(function) ?? throw new EngineException($"no function {function}");
        var input = JsonSerializer.SerializeToUtf8Bytes(args, Json);
        var at = alloc(input.Length);
        input.CopyTo(memory.GetSpan(at, input.Length));
        var packed = run(at, input.Length);
        var (place, size) = ((int)(packed >> 32), (int)(packed & 0xffffffff));
        var output = memory.GetSpan(place, size).ToArray();
        free(place, size);
        return output;
    }

    /// The services the engine uses, as module `clarity`, for one instance.
    Wasmtime.Linker Linker(Serving serving)
    {
        var linker = new Wasmtime.Linker(engine);
        if (uses.Contains("clock"))
            linker.DefineFunction("clarity", "clock", () => services.Clock ?? DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
        if (uses.Contains("random"))
            linker.DefineFunction("clarity", "random", () => Splitmix(ref serving.Random));
        if (uses.Contains("log"))
        {
            var log = services.Log ?? Console.Error.WriteLine;
            linker.DefineFunction("clarity", "log", (Wasmtime.Caller caller, int at, int len) =>
            {
                var memory = caller.GetMemory("memory") ?? throw new EngineException($"engine {name} has no memory");
                log($"engine {name}: {memory.ReadString(at, len, Encoding.UTF8)}");
            });
        }
        if (uses.Contains("language"))
        {
            // The language as it is at the call, written into the engine's room.
            var language = services.Language ?? Language;
            linker.DefineFunction("clarity", "language", (Wasmtime.Caller caller, int at, int room) =>
            {
                var memory = caller.GetMemory("memory") ?? throw new EngineException($"engine {name} has no memory");
                var bytes = Encoding.UTF8.GetBytes(language() ?? "");
                bytes.AsSpan(0, Math.Min(bytes.Length, Math.Max(room, 0))).CopyTo(memory.GetSpan(at, Math.Max(room, 0)));
                return bytes.Length;
            });
        }
        return linker;
    }

    /// Calls `function` and gives its value as the type code declares.
    public T Call<T>(string function, params object?[] args) => CallIn<T>(0, function, args);

    /// Calls `function` in open window `window` and gives its value as the type code declares.
    public T CallIn<T>(long window, string function, params object?[] args)
    {
        var value = CallIn(window, function, args);
        try
        {
            return value.Deserialize<T>(Json)!;
        }
        catch (JsonException e)
        {
            throw new EngineException($"engine {name} gave {function} a value of another type: {e.Message}");
        }
    }

    /// Whether a call has ended, for its timer.
    sealed class Running
    {
        public bool Done;

        /// Marks the call ended when disposed, after its timer.
        public IDisposable Finish() => new Ending(this);

        sealed class Ending(Running call) : IDisposable
        {
            public void Dispose()
            {
                lock (call)
                    call.Done = true;
            }
        }
    }

    /// A decimal as exact text, as Clarity's values travel.
    sealed class DecimalAsText : JsonConverter<decimal>
    {
        public override decimal Read(ref Utf8JsonReader reader, Type type, JsonSerializerOptions options) =>
            reader.TokenType == JsonTokenType.String
                ? decimal.Parse(reader.GetString()!, NumberStyles.Number, CultureInfo.InvariantCulture)
                : reader.GetDecimal();

        public override void Write(Utf8JsonWriter writer, decimal value, JsonSerializerOptions options) =>
            writer.WriteStringValue(value.ToString(CultureInfo.InvariantCulture));
    }
}
