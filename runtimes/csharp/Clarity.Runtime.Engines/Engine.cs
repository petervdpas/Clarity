// Engines (DESIGN 5.11): processing modules compiled once to WebAssembly,
// which every stack hosts alike (conformance/engines.json). An engine
// computes; it imports nothing, so it reaches no database, file or network.
// Each call runs on a fresh instance of the module, so an engine keeps no
// state from one call to the next. Values cross as Clarity's JSON:
// decimals as exact text, enums by name, records by their field names.

using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace Clarity.Runtime;

/// An engine that could not load, or that failed while it ran.
public sealed class EngineException(string what) : ClarityException(what);

public sealed class Engine
{
    readonly string name;
    readonly Wasmtime.Engine engine;
    readonly Wasmtime.Module module;

    Engine(string name, Wasmtime.Engine engine, Wasmtime.Module module)
    {
        this.name = name;
        this.engine = engine;
        this.module = module;
    }

    /// Values as Clarity writes them.
    public static readonly JsonSerializerOptions Json = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        Converters = { new DecimalAsText(), new JsonStringEnumConverter() },
    };

    /// Loads `bytes` as engine `name` with the functions it is declared with;
    /// refuses a module that is no engine, imports anything, or lacks one of them.
    public static Engine Load(string name, byte[] bytes, IReadOnlyList<string> functions)
    {
        EngineException Refused(string why) => new($"engine {name} {why}");
        if (bytes.Length < 4 || bytes[0] != 0 || bytes[1] != 'a' || bytes[2] != 's' || bytes[3] != 'm')
            throw Refused("is no WebAssembly module");
        var engine = new Wasmtime.Engine();
        Wasmtime.Module module;
        try
        {
            module = Wasmtime.Module.FromBytes(engine, name, bytes);
        }
        catch (Exception e)
        {
            throw Refused($"is no WebAssembly module: {e.Message}");
        }
        if (module.Imports.FirstOrDefault() is { } import)
            throw Refused($"imports {import.ModuleName}.{import.Name}; an engine imports nothing");
        var exported = module.Exports.Where(e => e is Wasmtime.FunctionExport).Select(e => e.Name).ToHashSet();
        foreach (var needed in new[] { "clarity_alloc", "clarity_free" })
            if (!exported.Contains(needed))
                throw Refused($"lacks {needed} of the engine interface");
        if (!module.Exports.Any(e => e is Wasmtime.MemoryExport && e.Name == "memory"))
            throw Refused("lacks the memory of the engine interface");
        if (functions.FirstOrDefault(f => !exported.Contains(f)) is { } missing)
            throw Refused($"has no function {missing}");
        return new Engine(name, engine, module);
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
            var instance = new Wasmtime.Linker(engine).Instantiate(store, module);
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
