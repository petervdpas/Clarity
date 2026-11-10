# Video player

Plays the mp4 videos of a folder the person chooses. A welcome video plays
when the window opens; after that, the folder's videos play one after
another.

## What you see and can do

The Main window holds one view, MainView:

- A folder picker at the top. The chosen folder is kept in the person's settings, and changing it stops the video and lists the new folder.
- The video, with a table of the folder's mp4 files beside it (also those in folders below it). Opening a row plays it.
- A progress slider under the video that follows it twice a second; moving it moves the video.
- The title of what plays, a volume slider (80 at the start), and **Previous**, **Play**, **Pause**, **Stop** and **Next**.

**Play** plays the video picked in the table, or the first one, and goes on
with a paused video. When a video ends, the next one starts. A video that
cannot be played shows an error with the reason.

## Stack

Go, desktop. An example builds only for the stack its project file names.

## What it shows

- The Video part: `play`, `pause`, `resume`, `stop`, `seek` and `volume`, its `playing`, `paused`, `position` and `length`, and its `onend` and `onfail` events.
- A file store from a setting (`filestore videos "setting:videoFolder"`), listed with `files f in videos where f.extension == "mp4"`.
- A record (`Clip`) for data that is not stored in the database.
- A Timer part that drives the progress slider.
- A video in `resources/` as the part's starting source.

## Files

- `../videoplayer.clarity`: the project file (title, version, stack, output folder, the SQLite data source, the video file store).
- `videoplayer.clarity`: the Clip record, the folder setting and the code behind the window and its view.
- `design/`: `Videoplayer.cxaml` (the app), `Main.cxaml` (the window), `MainView.cxaml` (the video, table, sliders and buttons).
- `resources/welcome.mp4`: the welcome video.
- `translations/`: English.
