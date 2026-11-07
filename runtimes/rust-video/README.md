# clarity-video

Clarity.Video: the player behind the `video` part of the programs Clarity generates
(docs/VIDEO-PLAN.md). It reads MP4 files and decodes their pictures through a codec interface.

- **openh264 is the base codec.** H.264 is decoded by Cisco's prebuilt openh264 library, which
  Clarity.Video fetches from Cisco the first time a video plays and keeps in the person's cache
  folder, checked against its known hashes. Cisco's binary is what keeps the H.264 royalties
  covered; a build from source would not be.
- **Codecs people add.** A codec is anything that implements `Codec` and registers itself
  (`clarity_video::register`); it wins over the base codec for the formats it takes.

## License

MIT. openh264 is Cisco's, under its own licence, fetched from Cisco and never carried.
