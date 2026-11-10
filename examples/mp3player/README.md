# Mp3 player

A music player for a folder of songs. You pick the folder, the app lists
its mp3, flac, ogg and wav files, and plays them one after another.

## What you see and can do

The Main window holds one view, MainView:

- A folder box at the top, bound to the `musicFolder` setting. Changing the folder stops the song and lists the new folder.
- A table of the songs, sortable and resizable. A double click or Enter plays the song; Play plays the picked song, or the first one.
- A line that says what plays. Once the file's tags are read it shows the title and the artist.
- A progress slider that follows the song. Moving it moves the song.
- A volume slider (0 to 100, default 80) with its value beside it, and a Mute button that turns into Unmute.
- Play, Pause, Stop, Prev and Next. Pause on a held song plays it on.

When a song ends the next one starts; after the last one the player stops.
A song that cannot be played gives an error message with the reason. A
folder without songs gives a message too.

The folder is a setting, so each person keeps their own on their machine.

## Stack

C#, desktop. An example builds only for the stack its project file names.

## What it shows

- Sound: a `Sound` part named `music` plays a file (`music.play`), with `pause`, `resume`, `stop`, `seek` and `volume`, and reads `position`, `length`, `title` and `artist`. Its `onend` and `onfail` run handlers.
- A `Timer` part that ticks every half second to move the progress bar.
- A file store on a setting: `filestore music "setting:musicFolder"`, listed with `files f in music where f.extension == "mp3" ...`.
- A record (`Track`) for data that lives only in the view, not in the database.

## Files

- `../mp3player.clarity`: the project file (stack, version, output folder, the SQLite data source, the `music` file store).
- `mp3player.clarity`: the Track record, the `musicFolder` setting and the handlers of Main and MainView.
- `design/`: `Mp3player.cxaml` (the app), `Main.cxaml` (the window), `MainView.cxaml` (the folder box, song table, sliders, buttons, sound and timer).
- `translations/`: English.
