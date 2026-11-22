# My Record Collection

A record collection in the style of the desktop programs of the 2000s:
albums with their tracks, artists, a tree of genres, and the albums you lend
to friends until they come back.

## What you see and can do

The app has a menu, a toolbar (Albums, Artists, On loan) and a status bar.
The menu holds File (Albums with Ctrl+1, Settings, Language, Theme, Quit
with Ctrl+Q), Collection (Artists, Genres, Overview) and Loans (On loan,
Friends).

- Albums opens first: a search box and a table of albums (title, artist, year, format, rating, price, tags), 25 to a page. The search finds words of the title, label, notes or tags, or part of the artist's name. Opening an album slides out its editor from the right: title, artist, genre, year, format, label and tags; tabs for the tracks and their total length, the purchase (date, price, rating, booklet file), the cover and notes. A track opens in a dialog. Use as artist photo copies the cover to the artist. Export list writes the titles to `collection.txt` in the exports folder.
- Artists: the artists as cards, searchable, with an editor that slides out (name, country, formed, website, photo). Find photos takes photos from the photos folder named after an artist (`Miles Davis.jpg`) for artists without one, when they are at least 200 pixels on each side. Back up photos copies every photo to the exports folder.
- Genres: a tree table of genres (Bebop under Jazz), edited in a dialog. A genre with genres below it is not removed.
- Overview: three charts of the collection: albums per format, albums per year as a line, and the money spent per genre as a pie.
- On loan: the open loans, or all of them with Show returned. Lend opens a dialog that starts today; Back marks the loan returned. An album that is with another friend cannot be lent again.
- Friends: a table of friends (name, phone, email), edited in a dialog.

A track longer than two hours is refused when saved. Settings is a dialog
with the exports folder, a setting each person keeps on their machine. The
two themes are Classic (grey and square, like a desktop program of 2003)
and Midnight (dark, with a green accent).

## Stack

Go, desktop. An example builds only for the stack its project file names.

## What it shows

- Data across modules: `music` holds the albums and `loans` uses it; `records` uses both.
- Field kinds: money, duration, date, url, email, image, file, `list<text>` tags, a reference to its own entity (the genre tree), and `search` fields for full-text search.
- Rules in code: `before save` and `before delete` that `refuse`, an action with `require`, a function with `ensure`.
- File stores: `photos` in a folder and `exports` on a setting, written and read by actions with declared `effects`.
- An engine: `engine Images from "images.wasm"` reads a picture's width and height. Its source is the Rust crate in `engines/images`.
- Charts bound to a list, grouped by a field and summed by a measure.
- Two themes with part looks and a named `primary` style.

## Files

- `../records.clarity`: the project file (stack, output folder, currency, the themes, the SQLite data source, the `photos` and `exports` file stores).
- `records.clarity`: the exports folder setting and the Classic and Midnight themes.
- `music.clarity`: Format, Genre, Artist, Album and Track, the Images engine, the photo and export actions, and the code of the album, artist and genre windows.
- `loans.clarity`: Friend and Loan, `bringBack`, and the code of the loan and friend windows.
- `design/`:
  - `Records.cxaml`: the app with its menu, toolbar and footer.
  - `AlbumBrowse.cxaml`, `AlbumBrowseView.cxaml`, `AlbumEditor.cxaml`, `TrackDialog.cxaml`: the albums, the album editor and a track.
  - `ArtistBrowse.cxaml`, `ArtistBrowseView.cxaml`, `ArtistEditor.cxaml`: the artists and their editor.
  - `GenreBrowse.cxaml`, `GenreBrowseView.cxaml`, `GenreDialog.cxaml`: the genre tree and a genre.
  - `CollectionOverview.cxaml`, `CollectionOverviewView.cxaml`: the charts.
  - `LoanBrowse.cxaml`, `LoanBrowseView.cxaml`, `LoanDialog.cxaml`: the loans and a loan.
  - `FriendBrowse.cxaml`, `FriendBrowseView.cxaml`, `FriendDialog.cxaml`: the friends and a friend.
  - `Preferences.cxaml`: the settings dialog.
- `diagrams/`: the data model as a Mermaid ER diagram and as a class diagram.
- `engines/images/`: the engine's Rust source; `engines/images.wasm` is the built engine the app loads.
- `resources/`: the logo shown above the albums.
- `translations/`: English, Dutch and German.
- `migrations/main/`: six steps, from the collection to camelCase names.
