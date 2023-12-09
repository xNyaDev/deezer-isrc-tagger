# Deezer ISRC automatic file tagger

deezer-isrc-tagger is a command line utility to tag audio files using the Deezer API with ISRC lookup. It will keep the
existing metadata unless `--clear` is passed, in which case it will remove all metadata except encoding information such
as encoding library.

The file has to be already tagged with the ISRC, or it can be provided as a command line argument `--isrc`.

deezer-isrc-tagger tries to find ISRC conflicts - some songs on Deezer with the same ISRC appear in multiple albums. If
that happens you will be prompted to choose the album to use.

deezer-isrc-tagger can automatically rename the file to `$MAIN_ARTISTS - $TITLE [$YEAR]` if the `--rename` flag is
passed. Some filenames might be not allowed on specific filesystems, which is why the following chars are changed:
- `*` to `＊`,
- `\ ` to `＼`,
- `:` to `：`,
- `"` to `＂`,
- `<` to `＜`,
- `>` to `＞`,
- `|` to `｜`,
- `?` to `？`,
- `/` to `／`.

The behaviour is emulating using [rclone encoding flag](https://rclone.org/overview/#encoding)
`Asterisk,BackSlash,Colon,DoubleQuote,LtGt,Pipe,Question,Slash`

For example usage run `deezer-isrc-tagger -h`