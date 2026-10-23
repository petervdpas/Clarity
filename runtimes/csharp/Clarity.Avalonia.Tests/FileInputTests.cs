// The file input (U1c): its name or that it has none, a chosen file and a
// removed one as changes, and the bytes of a stored file fetched only when
// asked for, through the real FileInput.

using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class FileInputTests
{
    [AvaloniaFact]
    public void It_shows_the_files_name_or_that_there_is_none()
    {
        var input = new FileInput { EmptyText = "No file" };
        Assert.Equal("No file", input.Shown);
        input.File = new RecordFile("manual.pdf", Picture.Stored);
        Assert.Equal("manual.pdf", input.Shown);
    }

    [AvaloniaFact]
    public void Choosing_and_removing_are_changes()
    {
        var input = new FileInput();
        var changed = 0;
        input.ValueChanged += (_, _) => changed++;
        input.Take("C:\\docs\\manual.pdf", [1, 2, 3]);
        Assert.Equal("manual.pdf", input.File?.Name);
        Assert.Equal([1, 2, 3], input.Bytes());
        input.Remove();
        Assert.Null(input.File);
        input.Remove();
        Assert.Equal(2, changed);
        input.IsReadOnly = true;
        input.Take("other.pdf", [4]);
        Assert.Null(input.File);
    }

    [AvaloniaFact]
    public void A_stored_file_is_fetched_only_when_asked_for()
    {
        var fetched = 0;
        var input = new FileInput { Fetch = () => { fetched++; return [9]; }, FetchFile = path => path == "ab/c.pdf" ? [7] : null };
        input.File = new RecordFile("manual.pdf", Picture.Stored);
        Assert.Equal(0, fetched);
        Assert.Equal([9], input.Bytes());
        Assert.Equal(1, fetched);
        input.File = new RecordFile("c.pdf", Picture.InStore("ab/c.pdf"));
        Assert.Equal([7], input.Bytes());
    }
}
