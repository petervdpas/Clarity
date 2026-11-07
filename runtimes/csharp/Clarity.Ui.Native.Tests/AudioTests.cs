// Clarity.Audio handed to Clarity.UI as a generated Program does; no song
// plays, so no speakers open.

public class AudioTests
{
    [Fact]
    public void ClarityAudioIsTakenByClarityUi()
    {
        Assert.True(Clarity.Audio.ClarityAudio.Use());
    }
}

public class VideoTests
{
    [Fact]
    public void ClarityVideoIsTakenByClarityUi()
    {
        Assert.True(Clarity.Video.ClarityVideo.Use());
    }

    /// <summary>AddCodec reads add_codec where clarity_video.h's table has it: after these functions.</summary>
    [Fact]
    public void AddCodecReadsTheTableWhereTheHeaderHasIt()
    {
        var header = File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "../../../../../rust-video-ffi/include/clarity_video.h"));
        var table = header[header.IndexOf("typedef struct ClarityVideoApi {")..header.IndexOf("} ClarityVideoApi;")];
        var functions = System.Text.RegularExpressions.Regex.Matches(table, @"\(\*(\w+)\)").Select(m => m.Groups[1].Value).ToList();
        Assert.Equal(Clarity.Video.ClarityVideo.FunctionsBeforeAddCodec, functions.IndexOf("add_codec"));
    }

    [Fact]
    public void ACodecLibraryThatIsNotThereIsNotAdded()
    {
        Assert.False(Clarity.Video.ClarityVideo.AddCodec("libnot-there.so"));
    }
}
