// Where a mirror of a window sends what the person does (DESIGN 7.6): the
// command, by its name, and what it was given; the server runs it on the
// window it keeps, and answers the window's state.

namespace Clarity.Avalonia;

public interface IRemote
{
    void Command(WindowViewModel window, string command, object?[] args);

    /// <summary>A picture or a file the window's <paramref name="fetcher"/> fetches from the server's database.</summary>
    Task<byte[]?> Fetch(WindowViewModel window, string fetcher, object?[] args);
}

/// <summary>
/// A method of a window the server may run for its mirror, as it runs a
/// [RelayCommand]: one that takes more than one value, such as what an
/// engine beside the screen or a custom part told.
/// </summary>
[AttributeUsage(AttributeTargets.Method)]
public sealed class CommandAttribute : Attribute;
