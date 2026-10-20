# Pictures and files

How an application shows fixed pictures from the images folder, keeps pictures in records (in the database or in a file store), and reads and writes files from code.

## Fixed pictures

The `images` folder of the module folder holds the application's fixed pictures, such as a logo. An image part shows one by its file name, given as its `source` (the **Source** property in the designer):

```clarity
// records/images/logo.png
page AlbumBrowse: window
    image at 12,1 span 1 rows 1 source "logo.png"
```

The picture must be in the `images` folder; a name that is not there is an error that lists what is. A picture is a `.png`, `.jpg`, `.gif`, `.bmp` or `.webp` file. Every generated application carries the folder, and the designer shows the pictures again when the folder changes. A picture keeps its proportions inside its part.

## Pictures in records

An `image` field holds a picture, or none. Without `store`, the picture is kept in the database. With `store`, it is a file in a file store, and the column holds the file's name there.

```clarity
entity Album
    cover: image, help "The scan of the front cover"

entity Artist
    photo: image, store photos
```

An image part bound to the field shows it, or "No picture" while there is none. While the pointer is on it, it offers a button to load a picture from a file and one to remove it.

```clarity
image name cover at 1,1 span 4 rows 6 bind album.cover
```

A table or list does not show a picture column. Reading records never carries their pictures; the screen fetches each one when it shows it. Any file that is not a picture is refused.

## File stores

A file store is a folder that the project file names with a `filestore` line. A relative folder is in the project's folder; `"env:NAME"` takes it from an environment variable, for a folder that differs per machine.

```clarity
filestore photos "data/photos"
filestore exports "env:SHOP_EXPORTS"
```

You can also manage them in the **File stores** section of **Settings** on the Project tab. In the entity editor, the **Store** column of an image field picks **database** or one of the stores.

In a store, a picture's file is named after the SHA-256 of its bytes, so the same picture is kept once. Files are never deleted, so a store can be a folder of photos that are not the application's alone. Moving a field between the database and a store does not move its pictures, and can lose them.

## Files in code

Code reaches files only in the file stores. `files f in store` lists every file of a store and its folders, and `where` and `order by` work as in `find`. A file has `name`, `path`, `extension`, `size` and `modified`.

```clarity
action export_collection()
    effects files.write
    let lines = ""
    for album in find a in Album order by a.title
        lines = lines + album.title + "\n"
    write lines to exports "collection.txt"
```

- `read text of f` gives the text a file holds; `read bytes of f` its contents as they are.
- `write t to exports "lists/albums.txt"` writes a file, making its folders, from text or from bytes. A path that would leave the store is refused.
- `artist.photo = f` makes a file a record's picture. In the field's own store the column holds its path and nothing is copied; anywhere else the picture is a copy.

## Effects

An action declares what it does with files: `files.read` to list or read, `files.write` to write. A function cannot reach files at all. Handlers can, as they can save. See [Functions and actions](code.functions.md) and [the language reference](../LANGUAGE.md).
