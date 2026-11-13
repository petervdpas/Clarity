# Pictures and files

The resources an application carries (fixed pictures, Markdown and other files), pictures and files in records (in the database or in a file store), file stores, and reading and writing files from code.

## Resources

The `resources` folder of the module folder holds the files the application carries: its fixed pictures, such as a logo, its Markdown files, such as the rules of a game, and any other file it needs, such as a PDF manual, a drawing (`.dxf`), a sound or a video. Each is a file with a name of its own; how they are named is up to the project (`rules-en.md`, `rules-nl.md`). Every generated application carries the folder, and the designer shows the resources again when the folder changes.

A part shows a resource in two ways:

- By its `source`, the same file in every language: `<Image source="logo.png" />`, `<Markdown source="license.md" />`. In the designer this is the **Source** property.
- By a key, a file per language: the part names the key (`markdown @gameRules`), and each language file points the key at the file that language shows (`gameRules resource "rules-nl.md"`). In the designer this is the **Key** property. See [Keys that point at resources](languages.md#keys-that-point-at-resources).

Resources sits under i18n in the project tree, where **+ New resource** adds one; see [Languages](languages.md).

The project's icon is a picture among the resources too; see [The project file](project.file.md).

## Fixed pictures

An image part shows a picture of the resources by its file name, or the picture its key points each language at (`image @bannerPicture`, with `bannerPicture resource "banner-nl.png"` in the Dutch file).

In the window's file, `records/design/AlbumBrowse.cxaml`, with the picture in `records/resources/logo.png`:

```xml
<Window class="records.AlbumBrowse" title="@albumBrowseTitle">
  <Image name="collectionLogo" at="12,1" span="1" rows="1" source="logo.png" />
</Window>
```

- The picture must be in the `resources` folder; a name that is not there is an error that lists the pictures that are: "image 'logo.png' is not in the resources folder; it has banner.png, cover.png".
- A picture is a `.png`, `.jpg`, `.jpeg`, `.gif`, `.bmp`, `.webp` or `.svg` file, which every stack shows without extra libraries; any other is refused: "'logo.tif' is not a picture every stack can show; use a .png, .jpg, .gif, .bmp, .webp or .svg file".
- The picture keeps its proportions inside its part.

## Markdown files

A markdown part shows a Markdown file of the resources: one file in every language by its `source`, or a file per language through a key. The key's value is a Markdown file in every language, never a text: long texts belong in the resources, not in the language files.

```xml
<Markdown name="rulesText" at="1,1" span="1" rows="12" caption="@gameRules" />
```

- A source that is no `.md` file is refused ("'rules.txt' is not a Markdown file; use a .md file"), and so is one that is not in the resources folder.
- A markdown part can show a text instead, bound to a field or a plain value of the window. It shows one of the three: a key, a source or what it binds.
- Every stack reads the same Markdown: headings (`#` to `######`), paragraphs, bullet and numbered items (indented two spaces per level), quotes (`> `, `>>` inside a quote), scene breaks (`***`, `---` or `___` alone on a line; a line of `---` under text is a break too, never an underlined heading), hard line breaks (two spaces or a backslash at a line's end), `**bold**`, `*italic*`, `***both***`, `` `code` ``, code blocks between three backticks, and links `[text](https://...)`. HTML is text. `conformance/markdown.json` gives each case.
- It reads like a page: room around the text, a column no wider than a reading width, from the left, headings by their level in the theme's heading font and size with space above them, quotes set in with a bar, a scene break as a short line in the middle.
- A link opens in the person's browser when it is an `http`, `https` or `mailto` address; with any other address only its text shows.
- The part wraps its text and scrolls when it is longer than the part.

To write Markdown with its preview, a memo takes `markdown`: Write and Preview stand on its top (Clarity's words `markdownWrite` and `markdownPreview`). Write is the memo, its Markdown colored as it is written in the text font, Ctrl+B and Ctrl+I making the selection bold and italic; Preview shows the text as a markdown part does. The parts themselves are in [Text parts](parts.text.md) and [Input parts](parts.input.md).

## Pictures in records

An `image` field holds a picture, or none. Without `store`, the picture is kept in the database. With `store`, it is a file in a file store, and the column holds the file's name there.

```clarity
entity Album
    cover: image, help @albumCoverHelp, label @albumCoverLabel

entity Artist
    photo: image, store photos, label @artistPhotoLabel
```

An image part bound to the field shows it, or its `noPicture` text while there is none. While the pointer is on it, it offers a button to load a picture from a file (`loadPicture`) and one to remove it (`removePicture`). A label or help text can be for it.

```xml
<Image name="cover" at="1,1" span="4" rows="6" bind="album.cover" />
```

- Reading records never carries their pictures, so a list of thousands stays light; the screen fetches each picture on its own when it shows it.
- A picture is a png, jpg, gif, bmp or webp file, told by its first bytes; any other file is refused.
- A table or list does not show a picture column; an image part does.
- A picture cannot be unique and has no default, and a field of a `record` cannot be one.
- In code, a picture is compared with `none` and set to `none`: `if album.cover != none`, `album.cover = none`. A query finds the records with or without one: `where a.cover == none`.
- It is set to another record's picture as well, which copies it, between the database and a file store too: the picture's bytes go along, and saving keeps them where the field keeps its pictures. It is set to a file of a file store too (see [Files in code](#files-in-code)).

```clarity
code AlbumEditor
    handler copyCover
        album.cover = other.cover
        artist.photo = album.cover
```

Moving a field's pictures between the database and a file store, or between `image` and another type, is a change that can lose data: pictures do not move by themselves.

## Files in records

A `file` field holds any file a record keeps, such as a manual, a contract or a spreadsheet, with its name, or none:

```clarity
entity Product
    manual: file, label @manualLabel
    sheet: file, store documents, label @sheetLabel
```

- Where it is kept: without `store`, the file is in the database, its bytes in the field's column and its name in a column beside it, `manual_name`, which migrations add, rename and remove with the field. With `store`, it is a file of that file store, in a folder named by its contents and under its own name (`3fa9c1.../datasheet.pdf`); the column holds that path, and the name column is there too. A field cannot become a file or stop being one; remove it and add a new field.
- The file part shows the file's name, or Clarity's word `noFile`, with buttons to choose a file, open it in the program the system uses for its kind, save a copy, and remove it. Choosing and removing are its `change`. A copy goes to the person's Downloads folder under the file's name (with ` (2)` and on when that name is taken there), and the part says so in Clarity's word `fileCopySaved`. Its buttons are Clarity's words `chooseFile`, `openFile`, `saveFileCopy` and `removeFile`.
- Reading records never carries the file itself, only its name; the file comes on its own when it is opened or copied.
- In code, `product.manual.name` and `product.manual.extension` read its name and extension; a file is compared with and set to `none`, set to a file of a store, and set to another record's file, `product.manual = other.manual`, which keeps its name.
- A file cannot be unique ("a file cannot be unique ('manual')") or have a default, and a table or list does not show a file column; a file part does.

```xml
<File name="manualBox" at="1,1" span="8" bind="product.manual" />
```

## File stores

A file store is a folder that the project file names with a `filestore` line. Code reaches files only in the file stores, the folders that also keep pictures and files of records: what a program touches on disk is in its project file.

```clarity
filestore photos "data/photos"
filestore exports "env:SHOP_EXPORTS"
filestore scans "setting:scansFolder"
```

- A relative folder is in the project's folder, as a SQLite file is.
- `"env:NAME"` takes it from that environment variable, for a folder that differs per machine.
- `"setting:name"` takes it from a folder setting that each person chooses while the application runs; see [Folder settings](data.settings.md#folder-settings).
- `store name` on an `image` or a `file` field keeps its pictures or files there; `store` on any other field is refused ("only an image or a file field has a store ('notes')").

You can also manage them in the **File stores** section of **Settings** on the Project tab. In the entity editor, the **Store** column of an image field picks **database** or one of the stores.

In a store, a picture's file is named after the SHA-256 of its bytes, so the same picture is kept once. Files are never deleted, so a store can be a folder of photos that are not the application's alone. Moving a field between the database and a store does not move its pictures, and can lose them.

## Files in code

```clarity
// Photos dropped in the photos folder, named after the artist
// (Miles Davis.jpg), become the photos of artists that have none.
action findArtistPhotos()
    effects db.write, files.read
    let found = files f in photos where f.extension == "jpg" or f.extension == "png"
    let artists = find a in Artist where a.photo == none
    for artist in artists
        for f in found
            if f.name == artist.name + "." + f.extension
                artist.photo = f
                save artist

action exportCollection()
    effects files.write
    let lines = ""
    for album in find a in Album order by a.title
        lines = lines + album.title + "\n"
    write lines to exports "collection.txt"
```

- `files f in store` lists every file of the store and its folders, leaving out files and folders whose name starts with a dot, sorted by path. `where` and `order by` work as in `find`, on the details of a file, and run in memory: the condition may be any bool. See [Queries](code.queries.md).
- A file has `name` (`Blue Train.jpg`), `path` inside the store (`2024/Blue Train.jpg`, with `/` between folders), `extension` in lower case without the dot (`jpg`), `size` in bytes and `modified` (a date and time, in UTC). A list of files is `list<file>`, one file `file`.
- `read text of f` gives the text a file holds, as UTF-8; `read bytes of f` its contents as they are, of type `bytes`.
- `write t to exports "lists/albums.txt"` writes a file, making its folders, from text or from bytes. A path that would leave the store is refused.
- `artist.photo = f` makes a file a record's picture. In the field's own store the column holds its path and nothing is copied, as a catalog of photos wants; anywhere else the picture is a copy. A file that is no picture is refused.

## Effects

An action declares what it does with files: `files.read` to list or read, `files.write` to write. A function cannot reach files at all. Handlers can, as they can save. See [Functions and actions](code.functions.md).

See also [Entities and enums](data.entities.md), [Settings](data.settings.md) and [Languages](languages.md).
