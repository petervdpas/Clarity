// The markdown part: Markdown read into blocks the same way on every stack
// (MarkdownBlocks), shown as headings, paragraphs, list items and code,
// wrapped, scrolling when longer than the part. Its text is a Markdown file
// of the application's resources (`Source`, by its name), or Markdown it is
// given (`Markdown`): a text it binds, or the file a key points at in the
// language shown. A link opens in the person's browser.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Documents;
using Avalonia.Input;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Platform;

namespace Clarity.Avalonia;

public class MarkdownView : UserControl
{
    public static readonly StyledProperty<string?> SourceProperty =
        AvaloniaProperty.Register<MarkdownView, string?>(nameof(Source));

    public static readonly StyledProperty<string?> MarkdownProperty =
        AvaloniaProperty.Register<MarkdownView, string?>(nameof(Markdown));

    /// A Markdown file of the resources folder, such as `rules.md`.
    public string? Source
    {
        get => GetValue(SourceProperty);
        set => SetValue(SourceProperty, value);
    }

    /// The Markdown itself, when the part binds a text.
    public string? Markdown
    {
        get => GetValue(MarkdownProperty);
        set => SetValue(MarkdownProperty, value);
    }

    /// The blocks shown, one control each.
    public StackPanel Blocks { get; } = new() { Spacing = 6 };

    /// Where each paragraph's links are: from which character, how many, to where.
    static readonly AttachedProperty<List<(int From, int Length, string Address)>?> LinksProperty =
        AvaloniaProperty.RegisterAttached<MarkdownView, TextBlock, List<(int, int, string)>?>("Links");

    public MarkdownView()
    {
        Content = new ScrollViewer
        {
            HorizontalScrollBarVisibility = global::Avalonia.Controls.Primitives.ScrollBarVisibility.Disabled,
            VerticalScrollBarVisibility = global::Avalonia.Controls.Primitives.ScrollBarVisibility.Auto,
            Content = Blocks,
        };
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == SourceProperty || change.Property == MarkdownProperty)
            Show();
    }

    /// Where the application's resource `name` is: its resources folder.
    /// <summary>
    /// The assembly the application's resources are in: the one of its App,
    /// which is the program itself on the desktop and the application's
    /// library in a browser (DESIGN 7.6), where the program is another.
    /// </summary>
    internal static string? AppAssembly =>
        global::Avalonia.Application.Current?.GetType().Assembly.GetName().Name ?? System.Reflection.Assembly.GetEntryAssembly()?.GetName().Name;

    public static Uri ResourceUri(string? assembly, string name) => new($"avares://{assembly}/resources/{name}");

    /// The text of a resource file of the application, or nothing.
    static string ReadResource(string name)
    {
        var uri = ResourceUri(AppAssembly, name);
        if (!AssetLoader.Exists(uri)) return "";
        using var reader = new StreamReader(AssetLoader.Open(uri));
        return reader.ReadToEnd();
    }

    void Show()
    {
        var text = Source is { Length: > 0 } source ? ReadResource(source) : Markdown ?? "";
        Blocks.Children.Clear();
        foreach (var block in MarkdownBlocks.Read(text))
            Blocks.Children.Add(Control(block));
    }

    Control Control(MarkdownBlock block)
    {
        switch (block.Kind)
        {
            case "heading":
                var heading = Paragraph(block.Spans);
                heading.FontWeight = FontWeight.SemiBold;
                heading.FontSize = FontSize * (block.Level switch { 1 => 1.4, 2 => 1.2, 3 => 1.05, _ => 1.0 });
                return heading;
            case "item":
                var marker = new TextBlock { Text = block.Number is int n ? $"{n}." : "•", TextAlignment = TextAlignment.Right, MinWidth = FontSize };
                var item = new Grid { ColumnDefinitions = new ColumnDefinitions("Auto,*"), Margin = new Thickness(block.Depth * 1.25 * FontSize, 0, 0, 0) };
                var words = Paragraph(block.Spans);
                words.Margin = new Thickness(FontSize * 0.5, 0, 0, 0);
                Grid.SetColumn(words, 1);
                item.Children.Add(marker);
                item.Children.Add(words);
                return item;
            case "code":
                return new Border
                {
                    Padding = new Thickness(8, 6),
                    CornerRadius = new CornerRadius(4),
                    Background = new SolidColorBrush(Colors.Gray, 0.12),
                    Child = new TextBlock { Text = block.Text, FontFamily = new FontFamily("monospace"), TextWrapping = TextWrapping.Wrap },
                };
            default:
                return Paragraph(block.Spans);
        }
    }

    /// The spans as one wrapped text, its links underlined and noted for the pointer.
    TextBlock Paragraph(IReadOnlyList<MarkdownSpan> spans)
    {
        var paragraph = new TextBlock { TextWrapping = TextWrapping.Wrap, Inlines = [] };
        var links = new List<(int, int, string)>();
        var at = 0;
        foreach (var span in spans)
        {
            var run = new Run(span.Text);
            if (span.Bold) run.FontWeight = FontWeight.SemiBold;
            if (span.Italic) run.FontStyle = FontStyle.Italic;
            if (span.Code)
            {
                run.FontFamily = new FontFamily("monospace");
                run.Background = new SolidColorBrush(Colors.Gray, 0.15);
            }
            if (span.Link is string address)
            {
                run.TextDecorations = TextDecorations.Underline;
                links.Add((at, span.Text.Length, address));
            }
            paragraph.Inlines.Add(run);
            at += span.Text.Length;
        }
        if (links.Count > 0)
        {
            paragraph.SetValue(LinksProperty, links);
            paragraph.PointerMoved += (_, e) => paragraph.Cursor = LinkUnder(paragraph, e) is null ? Cursor.Default : new Cursor(StandardCursorType.Hand);
            paragraph.PointerReleased += (_, e) =>
            {
                if (e.InitialPressMouseButton == MouseButton.Left && LinkUnder(paragraph, e) is string address)
                    Open(address);
            };
        }
        return paragraph;
    }

    /// The link at character `index` of a paragraph, if any.
    public static string? LinkAt(TextBlock paragraph, int index) =>
        paragraph.GetValue(LinksProperty)?.FirstOrDefault(l => index >= l.From && index < l.From + l.Length).Address;

    static string? LinkUnder(TextBlock paragraph, PointerEventArgs e)
    {
        var point = e.GetPosition(paragraph) - new Point(paragraph.Padding.Left, paragraph.Padding.Top);
        var hit = paragraph.TextLayout.HitTestPoint(point);
        return hit.IsInside ? LinkAt(paragraph, hit.TextPosition) : null;
    }

    void Open(string address)
    {
        if (Uri.TryCreate(address, UriKind.Absolute, out var uri))
            _ = TopLevel.GetTopLevel(this)?.Launcher.LaunchUriAsync(uri);
    }
}
