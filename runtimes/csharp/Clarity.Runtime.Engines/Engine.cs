// Engines (DESIGN 5.11): processing modules compiled once to WebAssembly,
// which every stack hosts alike (conformance/engines.json). An engine
// computes; it imports only the services of Clarity it declares (`uses
// clock, random, log`), so it reaches no database, file or network.
// Each call runs on a fresh instance of the module, so an engine keeps no
// state from one call to the next, and runs within the limits of its engine:
// a call that runs too long is stopped, and an engine never gets more memory
// than its limit. Values cross as Clarity's JSON:
// decimals as exact text, enums by name, records by their field names.

using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace Clarity.Runtime;

/// An engine that could not load, or that failed while it ran.
public sealed class EngineException(string what) : ClarityException(what);

/// What one call of an engine may take: how long it may run, and how much
/// memory the engine may grow to.
public readonly record struct EngineLimits(uint Seconds, uint Megabytes);

/// What the services give: the system's clock, a fresh seed for every call
/// and the standard error, unless a test fixes the clock (milliseconds since
/// 1970) or the seed, or keeps the lines of the log.
public sealed record EngineServices(long? Clock = null, ulong? Seed = null, Action<string>? Log = null);

public sealed class Engine
{
    readonly string name;
    readonly Wasmtime.Engine engine;
    readonly Wasmtime.Module module;
    readonly IReadOnlyList<string> uses;
    readonly EngineLimits limits;
    readonly EngineServices services;

    Engine(string name, Wasmtime.Engine engine, Wasmtime.Module module, IReadOnlyList<string> uses, EngineLimits limits, EngineServices services)
    {
        this.name = name;
        this.engine = engine;
        this.module = module;
        this.uses = uses;
        this.limits = limits;
        this.services = services;
    }

    /// The services an engine may use, each a function of module `clarity`
    /// with its parameters and results.
    static readonly Dictionary<string, (Wasmtime.ValueKind[] Parameters, Wasmtime.ValueKind[] Results)> Services = new()
    {
        ["clock"] = ([], [Wasmtime.ValueKind.Int64]),
        ["random"] = ([], [Wasmtime.ValueKind.Int64]),
        ["log"] = ([Wasmtime.ValueKind.Int32, Wasmtime.ValueKind.Int32], []),
    };

    /// The engine with `services` in place of the system's.
    public Engine WithServices(EngineServices services) => new(name, engine, module, uses, limits, services);

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
    public static Engine Load(string name, byte[] bytes, IReadOnlyList<string> functions, IReadOnlyList<string> uses, EngineLimits limits)
    {
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
        return new Engine(name, engine, module, uses, limits, new EngineServices());
    }

    /// Calls `function` with `args`: its value as JSON, the engine's refusal
    /// (a RefusedException in its own words), or what went wrong running it.
    public JsonNode? Call(string function, params object?[] args)
    {
        EngineException Failed(string why) => new($"engine {name} failed in {function}: {why}");
        byte[] output;
        try
        {
            using var store = new Wasmtime.Store(engine);
            store.SetLimits(memorySize: limits.Megabytes * 1024L * 1024L);
            // The epoch moves once the time is up, which stops the call at its
            // next check. The timer moves it only while this call runs, so a
            // late timer never cuts short the next call.
            store.SetEpochDeadline(1);
            var call = new Running();
            using var late = new Timer(_ =>
            {
                lock (call)
                    if (!call.Done)
                        engine.IncrementEpoch();
            }, null, TimeSpan.FromSeconds(limits.Seconds), Timeout.InfiniteTimeSpan);
            using var finish = call.Finish();
            using var linker = Linker();
            var instance = linker.Instantiate(store, module);
            var memory = instance.GetMemory("memory") ?? throw Failed("no memory");
            var alloc = instance.GetFunction<int, int>("clarity_alloc") ?? throw Failed("no clarity_alloc");
            var free = instance.GetAction<int, int>("clarity_free") ?? throw Failed("no clarity_free");
            var run = instance.GetFunction<int, int, long>(function) ?? throw Failed("no such function");
            var input = JsonSerializer.SerializeToUtf8Bytes(args, Json);
            var at = alloc(input.Length);
            input.CopyTo(memory.GetSpan(at, input.Length));
            var packed = run(at, input.Length);
            var (place, size) = ((int)(packed >> 32), (int)(packed & 0xffffffff));
            output = memory.GetSpan(place, size).ToArray();
            free(place, size);
        }
        catch (EngineException)
        {
            throw;
        }
        catch (Wasmtime.TrapException e) when (e.Type == Wasmtime.TrapCode.Interrupt)
        {
            throw Failed($"it ran longer than {limits.Seconds} second{(limits.Seconds == 1 ? "" : "s")}");
        }
        catch (Exception e)
        {
            throw Failed(e.Message);
        }
        JsonNode? answer;
        try
        {
            answer = JsonNode.Parse(Encoding.UTF8.GetString(output));
        }
        catch (JsonException e)
        {
            throw Failed($"its answer is no JSON: {e.Message}");
        }
        if (answer?["error"] is { } error)
            throw new RefusedException(Text.Plain(error.GetValue<string>()));
        if (answer is not JsonObject found || !found.ContainsKey("value"))
            throw Failed("its answer has no value");
        return found["value"];
    }

    /// The services the engine uses, as module `clarity`, for one call.
    Wasmtime.Linker Linker()
    {
        var linker = new Wasmtime.Linker(engine);
        if (uses.Contains("clock"))
            linker.DefineFunction("clarity", "clock", () => services.Clock ?? DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
        if (uses.Contains("random"))
        {
            var state = services.Seed ?? (ulong)Random.Shared.NextInt64(long.MinValue, long.MaxValue);
            linker.DefineFunction("clarity", "random", () => Splitmix(ref state));
        }
        if (uses.Contains("log"))
        {
            var log = services.Log ?? Console.Error.WriteLine;
            linker.DefineFunction("clarity", "log", (Wasmtime.Caller caller, int at, int len) =>
            {
                var memory = caller.GetMemory("memory") ?? throw new EngineException($"engine {name} has no memory");
                log($"engine {name}: {memory.ReadString(at, len, Encoding.UTF8)}");
            });
        }
        return linker;
    }

    /// Calls `function` and gives its value as the type code declares.
    public T Call<T>(string function, params object?[] args)
    {
        var value = Call(function, args);
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
