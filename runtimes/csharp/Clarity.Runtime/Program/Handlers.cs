// How a handler call the library sends runs on a generated view model (the
// logic of ViewModelHost before the library drew the windows): a handler
// without values by its On<Handler> through the view model's Handle, which
// says whether it refused; a call with values by the Run_ the generator
// writes beside its command (Run_Digit7); a command where there is nothing
// else; a click that names a window by its Open_ command. Each is found by
// the name the C# generator gives it (crates/clarity-gen/src/csharp/windows.rs).

using System.Collections.Concurrent;
using System.Reflection;
using System.Windows.Input;

namespace Clarity.Runtime;

internal static class Handlers
{
    /// <summary>
    /// Runs <paramref name="written"/> (<c>approve</c>, <c>digit(7)</c>) on
    /// <paramref name="model"/>: true when it ran and did not refuse. A call
    /// the window has no command for says so among its messages.
    /// </summary>
    public static bool Run(WindowViewModel model, string written)
    {
        var (handler, args) = Members.Call(written);
        var shape = Shape.Of(model.GetType());
        if (args.Count == 0 && Handler(model, shape, handler) is { } own)
            return model.Handle(own, null, CommandName(shape, handler));
        if (args.Count > 0 && RunOf(model, shape, handler, args) is { } withValues)
            return withValues();
        if (Command(model, shape, handler, args) is { } command)
        {
            if (!command.CanExecute(null))
                return false;
            command.Execute(null);
            return true;
        }
        Console.Error.WriteLine($"{WindowName(model)} runs {written}, which the window has no command for");
        model.Say(Level.Error, Text.Plain(Locale.Translate("handlerMissing", "This does nothing yet: the window has no handler for it.")));
        return false;
    }

    /// <summary>Whether the view model has a command for <paramref name="written"/>.</summary>
    public static bool Has(WindowViewModel model, string written)
    {
        var (handler, args) = Members.Call(written);
        return Command(model, Shape.Of(model.GetType()), handler, args) is not null;
    }

    /// <summary>The view model's name as its markup goes by: MainViewModel is the window Main.</summary>
    public static string WindowName(WindowViewModel model) => WindowName(model.GetType());

    public static string WindowName(Type type)
    {
        var name = type.Name;
        return name.EndsWith("ViewModel", StringComparison.Ordinal) ? name[..^"ViewModel".Length] : name;
    }

    /// <summary>The view model's own On&lt;Handler&gt; of a handler without values, which its command runs; none for a target or a call.</summary>
    static Action? Handler(WindowViewModel model, Shape shape, string handler)
    {
        var name = Members.Pascal(handler);
        if (Named(model, shape, CommandName(shape, handler)) is null)
            return null;
        // The generator steps around a member every view model has: `open` runs OnOpen_.
        if (!shape.Handlers.Contains(name))
            name += "_";
        if (!shape.Handlers.Contains(name))
            return null;
        var method = model.GetType().GetMethod("On" + name, BindingFlags.NonPublic | BindingFlags.Public | BindingFlags.Instance, Type.EmptyTypes);
        return method is { ReturnType.Name: "Void" } ? method.CreateDelegate<Action>(model) : null;
    }

    /// <summary>The command of a handler call: its handler's, its call's with values, or a window it opens.</summary>
    static ICommand? Command(WindowViewModel model, Shape shape, string handler, List<string> args)
    {
        if (args.Count == 0)
            return Named(model, shape, CommandName(shape, handler)) ?? Target(model, shape, handler);
        return CommandOf(model, shape, handler, args) is { } name ? Named(model, shape, name) : null;
    }

    /// <summary>The name of the command of a call with values, as the generator wrote it; the generator numbers a name that is taken (Digit7, then Digit72).</summary>
    static string? CommandOf(WindowViewModel model, Shape shape, string handler, List<string> args)
    {
        var name = Members.CallCommand(handler, args);
        for (var n = 1; n < 10; n++)
        {
            var numbered = n == 1 ? name : $"{name}{n}";
            if (Named(model, shape, numbered) is not null)
                return numbered;
        }
        return null;
    }

    /// <summary>The Run_ of a call with values (Run_Digit7), which runs it and says whether its handler refused.</summary>
    static Func<bool>? RunOf(WindowViewModel model, Shape shape, string handler, List<string> args)
    {
        if (CommandOf(model, shape, handler, args) is not { } name)
            return null;
        var method = model.GetType().GetMethod("Run_" + name, BindingFlags.Public | BindingFlags.Instance, Type.EmptyTypes);
        return method is { ReturnType.Name: "Boolean" } ? method.CreateDelegate<Func<bool>>(model) : null;
    }

    static ICommand? Named(WindowViewModel model, Shape shape, string command) =>
        shape.Commands.GetValueOrDefault(command)?.GetValue(model) as ICommand;

    /// <summary>The command of a handler without values: <c>RedInk</c>, or <c>RedInk_</c> where the generator stepped around a name.</summary>
    static string CommandName(Shape shape, string handler)
    {
        var name = Members.Pascal(handler);
        return shape.Commands.ContainsKey(name) ? name : shape.Commands.ContainsKey(name + "_") ? name + "_" : name;
    }

    /// <summary>What a click opens: a window, in place or in a window of its own, or shows a popover.</summary>
    static ICommand? Target(WindowViewModel model, Shape shape, string target) =>
        Named(model, shape, $"Open_{target}") ?? Named(model, shape, $"OpenNew_{target}") ?? Named(model, shape, $"Show_{target}");

    /// <summary>What a view model type has, found once: its commands and its handlers.</summary>
    sealed class Shape
    {
        static readonly ConcurrentDictionary<Type, Shape> shapes = new();

        public required Dictionary<string, PropertyInfo> Commands { get; init; }
        public required HashSet<string> Handlers { get; init; }

        public static Shape Of(Type type) => shapes.GetOrAdd(type, Read);

        static Shape Read(Type type)
        {
            var properties = type.GetProperties(BindingFlags.Public | BindingFlags.Instance);
            var handlers = type.GetMethods(BindingFlags.NonPublic | BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly)
                .Where(m => m.Name.StartsWith("On", StringComparison.Ordinal) && m.Name.Length > 2 && m.GetBaseDefinition() == m)
                .Select(m => m.Name[2..]);
            return new Shape
            {
                Commands = properties
                    .Where(p => typeof(ICommand).IsAssignableFrom(p.PropertyType) && p.Name.EndsWith("Command", StringComparison.Ordinal))
                    .ToDictionary(p => p.Name[..^"Command".Length]),
                Handlers = [.. handlers],
            };
        }
    }
}
