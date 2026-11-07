package clarity

import (
	"bytes"
	"encoding/binary"
	"hash/crc32"
)

// Zip is zip(paths, texts) (conformance/zip.json): a zip archive of one file
// per path holding the text of the same place, stored as it is, in order, so
// an EPUB's mimetype can come first. Paths and texts pair up; what the longer
// list has more is left out. The same files give the same bytes.
func Zip(paths, texts []string) []byte {
	var out, central bytes.Buffer
	put := func(b *bytes.Buffer, values ...any) {
		for _, v := range values {
			_ = binary.Write(b, binary.LittleEndian, v)
		}
	}
	count := 0
	for i := 0; i < len(paths) && i < len(texts); i++ {
		name, data := []byte(paths[i]), []byte(texts[i])
		crc, size, offset := crc32.ChecksumIEEE(data), uint32(len(data)), uint32(out.Len())
		// Version 20, UTF-8 names, stored, 1980-01-01 00:00.
		put(&out, uint32(0x04034b50), uint16(20), uint16(0x0800), uint16(0), uint16(0), uint16(0x21), crc, size, size, uint16(len(name)), uint16(0))
		out.Write(name)
		out.Write(data)
		put(&central, uint32(0x02014b50), uint16(20), uint16(20), uint16(0x0800), uint16(0), uint16(0), uint16(0x21), crc, size, size, uint16(len(name)), uint16(0), uint16(0), uint16(0), uint16(0), uint32(0), offset)
		central.Write(name)
		count++
	}
	centralSize, centralAt := uint32(central.Len()), uint32(out.Len())
	out.Write(central.Bytes())
	put(&out, uint32(0x06054b50), uint16(0), uint16(0), uint16(count), uint16(count), centralSize, centralAt, uint16(0))
	return out.Bytes()
}
