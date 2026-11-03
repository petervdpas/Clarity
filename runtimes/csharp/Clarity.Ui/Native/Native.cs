// The C interface of the native Clarity.UI library (clarity_ui.h, DESIGN
// 7.9), as P/Invoke calls with source-generated marshalling. Text goes in as
// UTF-8 bytes with their length; text the library gives back is freed with
// clarity_string_free. Every call returns a status, which Check turns into a
// ClarityException with the library's message.

using System.Runtime.InteropServices;

namespace Clarity.Ui.Native;

/// <summary>What a call of the library answered (CLARITY_OK and the others).</summary>
public enum Status
{
    Ok = 0,
    Failed = 1,
    Panic = 2,
    Invalid = 3,
    NotFound = 4,
}

/// <summary>The program's callbacks (ClarityProgramCallbacks): one call by command, and its release.</summary>
[StructLayout(LayoutKind.Sequential)]
internal unsafe struct ProgramCallbacks
{
    public nint User;
    public delegate* unmanaged[Cdecl]<nint, byte*, nuint, byte*, nuint, nint, void> Call;
    public delegate* unmanaged[Cdecl]<nint, void> Release;
}

/// <summary>Where a custom part is pointed at (ClarityPointer).</summary>
[StructLayout(LayoutKind.Sequential)]
internal struct NativePointer
{
    public double X;
    public double Y;
    public uint Modifiers;
    public uint Clicks;
}

/// <summary>A custom part's callbacks (ClarityCustomCallbacks), in the header's order.</summary>
[StructLayout(LayoutKind.Sequential)]
internal unsafe struct CustomCallbacks
{
    public nint User;
    public delegate* unmanaged[Cdecl]<nint, nint> Make;
    public delegate* unmanaged[Cdecl]<nint, byte*, nuint, byte*, nuint, nint, void> Set;
    public delegate* unmanaged[Cdecl]<nint, byte*, nuint, byte*, nuint, nint, void> Hear;
    public delegate* unmanaged[Cdecl]<nint, nint, double, double, double, double, void> Draw;
    public delegate* unmanaged[Cdecl]<nint, nint, void> Start;
    public delegate* unmanaged[Cdecl]<nint, int> Focusable;
    public delegate* unmanaged[Cdecl]<nint, NativePointer*, nint, void> PointerPressed;
    public delegate* unmanaged[Cdecl]<nint, NativePointer*, nint, void> PointerMoved;
    public delegate* unmanaged[Cdecl]<nint, NativePointer*, nint, void> PointerReleased;
    public delegate* unmanaged[Cdecl]<nint, nint, void> Click;
    public delegate* unmanaged[Cdecl]<nint, double, double, uint, nint, int> Wheel;
    public delegate* unmanaged[Cdecl]<nint, byte*, nuint, uint, nint, int> Key;
    public delegate* unmanaged[Cdecl]<nint, byte*, nuint, nint, void> Text;
    public delegate* unmanaged[Cdecl]<nint, int, nint, void> FocusChanged;
    public delegate* unmanaged[Cdecl]<nint, nint, void> Wake;
    public delegate* unmanaged[Cdecl]<nint, void> Stop;
    public delegate* unmanaged[Cdecl]<nint, double, double, nint, void> TooltipAt;
    public delegate* unmanaged[Cdecl]<nint, void> Destroy;
}

/// <summary>The library's functions, by their C names.</summary>
internal static unsafe partial class Lib
{
    /// <summary>The library's name, as each system finds it: libclarity_ui.so, clarity_ui.dll, libclarity_ui.dylib.</summary>
    public const string Name = "clarity_ui";

    /// <summary>The interface version this layer is written for (CLARITY_UI_INTERFACE).</summary>
    public const uint Interface = 1;

    [LibraryImport(Name, EntryPoint = "clarity_last_error")]
    public static partial byte* LastError();

    [LibraryImport(Name, EntryPoint = "clarity_string_free")]
    public static partial void StringFree(byte* text);

    [LibraryImport(Name, EntryPoint = "clarity_ui_version")]
    public static partial byte* Version();

    [LibraryImport(Name, EntryPoint = "clarity_ui_interface")]
    public static partial uint InterfaceVersion();

    [LibraryImport(Name, EntryPoint = "clarity_answer_set")]
    public static partial Status AnswerSet(nint answer, byte* data, nuint len);

    // -- The program's files ------------------------------------------------------

    [LibraryImport(Name, EntryPoint = "clarity_files_new")]
    public static partial Status FilesNew(out nint files);

    [LibraryImport(Name, EntryPoint = "clarity_files_free")]
    public static partial void FilesFree(nint files);

    [LibraryImport(Name, EntryPoint = "clarity_files_window")]
    public static partial Status FilesWindow(nint files, byte* name, nuint nameLen, byte* markup, nuint markupLen);

    [LibraryImport(Name, EntryPoint = "clarity_files_app")]
    public static partial Status FilesApp(nint files, byte* name, nuint nameLen, byte* markup, nuint markupLen);

    [LibraryImport(Name, EntryPoint = "clarity_files_theme")]
    public static partial Status FilesTheme(nint files, byte* name, nuint nameLen, byte* json, nuint jsonLen);

    [LibraryImport(Name, EntryPoint = "clarity_files_model")]
    public static partial Status FilesModel(nint files, byte* json, nuint jsonLen);

    [LibraryImport(Name, EntryPoint = "clarity_files_resource")]
    public static partial Status FilesResource(nint files, byte* name, nuint nameLen, byte* bytes, nuint bytesLen);

    [LibraryImport(Name, EntryPoint = "clarity_files_title")]
    public static partial Status FilesTitle(nint files, byte* title, nuint titleLen);

    [LibraryImport(Name, EntryPoint = "clarity_files_size")]
    public static partial Status FilesSize(nint files, double width, double height);

    // -- Language and theme -------------------------------------------------------------

    [LibraryImport(Name, EntryPoint = "clarity_language_choose")]
    public static partial Status LanguageChoose(byte* tag, nuint tagLen);

    [LibraryImport(Name, EntryPoint = "clarity_translate")]
    public static partial Status Translate(byte* key, nuint keyLen, byte* english, nuint englishLen, out byte* text);

    // -- The program ----------------------------------------------------------------

    [LibraryImport(Name, EntryPoint = "clarity_program_new")]
    public static partial Status ProgramNew(ProgramCallbacks* callbacks, out nint program);

    [LibraryImport(Name, EntryPoint = "clarity_program_free")]
    public static partial void ProgramFree(nint program);

    [LibraryImport(Name, EntryPoint = "clarity_desktop_run")]
    public static partial Status DesktopRun(nint program, nint files, out int exitCode);

    [LibraryImport(Name, EntryPoint = "clarity_desktop_wake")]
    public static partial void DesktopWake();

    [LibraryImport(Name, EntryPoint = "clarity_driven_start")]
    public static partial Status DrivenStart(nint program, nint files, out nint driven);

    [LibraryImport(Name, EntryPoint = "clarity_driven_step")]
    public static partial Status DrivenStep(nint driven, byte* step, nuint stepLen, out byte* answer);

    [LibraryImport(Name, EntryPoint = "clarity_driven_free")]
    public static partial void DrivenFree(nint driven);

    // -- Custom parts -------------------------------------------------------------------

    [LibraryImport(Name, EntryPoint = "clarity_custom_register")]
    public static partial Status CustomRegister(byte* kind, nuint kindLen, CustomCallbacks* callbacks);

    [LibraryImport(Name, EntryPoint = "clarity_custom_bounds")]
    public static partial Status CustomBounds(nint cx, out double x, out double y, out double width, out double height);

    [LibraryImport(Name, EntryPoint = "clarity_custom_enabled")]
    public static partial Status CustomEnabled(nint cx, out int enabled);

    [LibraryImport(Name, EntryPoint = "clarity_custom_invalidate")]
    public static partial Status CustomInvalidate(nint cx);

    [LibraryImport(Name, EntryPoint = "clarity_custom_wake_in")]
    public static partial Status CustomWakeIn(nint cx, double ms);

    [LibraryImport(Name, EntryPoint = "clarity_custom_animate")]
    public static partial Status CustomAnimate(nint cx, int on);

    [LibraryImport(Name, EntryPoint = "clarity_custom_focus")]
    public static partial Status CustomFocus(nint cx);

    [LibraryImport(Name, EntryPoint = "clarity_custom_change")]
    public static partial Status CustomChange(nint cx, byte* value, nuint valueLen);

    [LibraryImport(Name, EntryPoint = "clarity_custom_tell")]
    public static partial Status CustomTell(nint cx, byte* tell, nuint tellLen, byte* values, nuint valuesLen);

    // -- The canvas a custom part draws on -------------------------------------------------

    [LibraryImport(Name, EntryPoint = "clarity_canvas_fill")]
    public static partial Status CanvasFill(nint canvas, double x, double y, double width, double height, uint color, double radius);

    [LibraryImport(Name, EntryPoint = "clarity_canvas_stroke")]
    public static partial Status CanvasStroke(nint canvas, double x, double y, double width, double height, uint color, double lineWidth, double radius);

    [LibraryImport(Name, EntryPoint = "clarity_canvas_line")]
    public static partial Status CanvasLine(nint canvas, double x1, double y1, double x2, double y2, uint color, double lineWidth);

    [LibraryImport(Name, EntryPoint = "clarity_canvas_oval")]
    public static partial Status CanvasOval(nint canvas, double x, double y, double width, double height, uint color, int filled, double lineWidth);

    [LibraryImport(Name, EntryPoint = "clarity_canvas_path")]
    public static partial Status CanvasPath(nint canvas, byte* svg, nuint svgLen, uint color, int filled, double lineWidth);

    [LibraryImport(Name, EntryPoint = "clarity_canvas_text")]
    public static partial Status CanvasText(nint canvas, byte* text, nuint textLen, double x, double y, double width, double height, uint color, double size, int bold, int align);

    [LibraryImport(Name, EntryPoint = "clarity_canvas_measure")]
    public static partial Status CanvasMeasure(nint canvas, byte* text, nuint textLen, double size, int bold, out double width);

    [LibraryImport(Name, EntryPoint = "clarity_canvas_pixels")]
    public static partial Status CanvasPixels(nint canvas, byte* rgba, uint width, uint height, nuint stride, double x, double y, double boxWidth, double boxHeight);

    [LibraryImport(Name, EntryPoint = "clarity_canvas_clip")]
    public static partial Status CanvasClip(nint canvas, double x, double y, double width, double height);

    [LibraryImport(Name, EntryPoint = "clarity_canvas_restore")]
    public static partial Status CanvasRestore(nint canvas);

    [LibraryImport(Name, EntryPoint = "clarity_canvas_look_color")]
    public static partial Status CanvasLookColor(nint canvas, byte* role, nuint roleLen, out uint color);
}
