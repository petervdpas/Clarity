// What a window says when code fails at a value or a record: Clarity's own
// words by a literal key (translated where the project has them), while the
// exception's message stays the English of the shared cases.

using System.Reflection;
using System.Text.RegularExpressions;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class RuntimeErrorTextTests
{
    [Fact]
    public void An_item_outside_its_list_is_said_by_its_key()
    {
        var failed = Assert.Throws<ValueException>(() => Lists.At(new List<long> { 1 }, 10));
        Assert.Equal("there is no item 10 in a list of 1", failed.Message);
        Assert.Equal("itemOutsideList", failed.Said.Key);
        Assert.Equal("There is no item 10 in a list of 1", failed.Said.Show());
    }

    [Fact]
    public void Whole_numbers_divide_toward_zero_and_by_zero_is_said_by_its_key()
    {
        Assert.Equal((-3L, -1L, long.MinValue), (Lists.WholeDiv(-7, 2), Lists.WholeMod(-7, 3), Lists.WholeDiv(long.MinValue, -1)));
        var failed = Assert.Throws<ValueException>(() => Lists.WholeDiv(1, 0));
        Assert.Equal("division by zero", failed.Message);
        Assert.Equal("divisionByZero", failed.Said.Key);
        Assert.Throws<ValueException>(() => Lists.WholeMod(1, 0));
    }

    [Fact]
    public void A_record_not_there_or_in_use_is_said_by_its_key()
    {
        Assert.Equal("recordNotThere", new NotFoundException("Customer", 3).Said.Key);
        Assert.Equal("There is no Customer with id 3", new NotFoundException("Customer", 3).Said.Show());
        Assert.Equal("recordStillUsed", new InUseException("Customer", 3).Said.Key);
    }

    [Fact]
    public void A_window_says_the_failure_in_Clarity_s_own_words()
    {
        var window = new ShelfViewModel(null);
        window.Handle(() => Lists.At(new List<long> { 1 }, 10));
        Assert.Equal("itemOutsideList", Assert.Single(window.Messages).Text.Key);
    }

    /// <summary>
    /// The generator steps around every member a window's view model has
    /// (names.rs VIEW_MODEL_MEMBERS), so a part, data or a handler's command
    /// never hides one (CS0108) or clashes with it: the list holds them all.
    /// </summary>
    [Fact]
    public void The_generator_knows_every_member_of_a_window_view_model()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null && !File.Exists(Path.Combine(dir.FullName, "crates", "clarity-gen", "src", "csharp", "names.rs")))
            dir = dir.Parent;
        Assert.NotNull(dir);
        var source = File.ReadAllText(Path.Combine(dir!.FullName, "crates", "clarity-gen", "src", "csharp", "names.rs"));
        var list = Regex.Match(source, @"VIEW_MODEL_MEMBERS: \[&str; \d+\] = \[(?<names>[^\]]*)\]").Groups["names"].Value;
        var known = Regex.Matches(list, "\"(\\w+)\"").Select(m => m.Groups[1].Value).ToHashSet();
        const BindingFlags all = BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static;
        var members = typeof(WindowViewModel).GetMembers(all)
            .Where(m => m switch
            {
                MethodBase method => (method.IsPublic || method.IsFamily || method.IsFamilyOrAssembly) && !method.IsSpecialName && !method.IsConstructor,
                PropertyInfo property => property.GetMethod is { } get && (get.IsPublic || get.IsFamily || get.IsFamilyOrAssembly),
                EventInfo ev => ev.AddMethod is { } add && (add.IsPublic || add.IsFamily),
                _ => false,
            })
            .Select(m => m.Name.Split('`')[0])
            .Distinct();
        Assert.DoesNotContain(members, m => !known.Contains(m));
    }
}
