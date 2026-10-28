// Where a mirror of a window sends what the person does (DESIGN 7.6): the
// command, by its name, and what it was given; the server runs it on the
// window it keeps, and answers the window's state.

namespace Clarity.Avalonia;

public interface IRemote
{
    void Command(WindowViewModel window, string command, object?[] args);
}
