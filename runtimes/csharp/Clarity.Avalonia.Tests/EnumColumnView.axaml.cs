using Avalonia.Controls;
using Avalonia.Markup.Xaml;

namespace Clarity.Avalonia.Tests;

public partial class EnumColumnView : UserControl
{
    public EnumColumnView() => AvaloniaXamlLoader.Load(this);
}

public enum Size
{
    Small,
    Large,
}

/// <summary>An enum's choices, as generated code writes them.</summary>
public static class SizeChoices
{
    public static Choice[] Size => [new(Tests.Size.Small, "Small"), new(Tests.Size.Large, "Large")];
}
