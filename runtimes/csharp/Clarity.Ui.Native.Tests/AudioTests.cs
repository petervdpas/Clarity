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
