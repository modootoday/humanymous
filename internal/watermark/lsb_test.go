package watermark

import (
	"bytes"
	"image"
	"image/color"
	"image/png"
	"testing"
)

// realPNG builds a small non-trivial PNG with enough pixels for the LSB channel.
func realPNG(t *testing.T) []byte {
	t.Helper()
	img := image.NewRGBA(image.Rect(0, 0, 64, 64))
	for y := 0; y < 64; y++ {
		for x := 0; x < 64; x++ {
			img.Set(x, y, color.RGBA{uint8(x * 3), uint8(y * 3), uint8((x + y) * 2), 255})
		}
	}
	var buf bytes.Buffer
	if err := png.Encode(&buf, img); err != nil {
		t.Fatal(err)
	}
	return buf.Bytes()
}

func TestLSB_SurvivesMetadataStrip(t *testing.T) {
	src := realPNG(t)
	p := Payload{TagID: 99, SessionID: "sess-lsb", AssetID: "/photo.png", Bucket: 20300}

	// Watermark with the composite (tEXt + LSB).
	out, _, err := Apply(mk, "image/png", src, p)
	if err != nil {
		t.Fatal(err)
	}

	// Attacker strips ALL metadata by decoding + re-encoding the pixels only.
	stripped := reencodePixelsOnly(t, out)

	// The tEXt chunk is gone, but the LSB payload must survive.
	carrier, channel, err := Recover("image/png", stripped)
	if err != nil {
		t.Fatalf("watermark lost after metadata strip: %v", err)
	}
	if channel != "png-lsb" && channel != "png-multi" {
		t.Logf("recovered via channel %s", channel)
	}
	id, tag, err := SplitCarrier(carrier)
	if err != nil || id != 99 || !Verify(mk, p, tag) {
		t.Fatalf("carrier invalid after strip (id=%d): %v", id, err)
	}
}

// reencodePixelsOnly decodes a PNG and re-encodes only the pixels, discarding
// all ancillary chunks (tEXt) — the metadata-strip attack.
func reencodePixelsOnly(t *testing.T, src []byte) []byte {
	t.Helper()
	img, err := png.Decode(bytes.NewReader(src))
	if err != nil {
		t.Fatal(err)
	}
	var buf bytes.Buffer
	if err := png.Encode(&buf, img); err != nil {
		t.Fatal(err)
	}
	return buf.Bytes()
}

func TestLSB_DirectRoundTrip(t *testing.T) {
	src := realPNG(t)
	framed := Frame(Carrier(mk, Payload{TagID: 5, SessionID: "s", AssetID: "/a", Bucket: 1}))
	c := lsbCodec{}
	out, err := c.Encode(src, framed)
	if err != nil {
		t.Fatal(err)
	}
	got, err := c.Decode(out)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(got, Carrier(mk, Payload{TagID: 5, SessionID: "s", AssetID: "/a", Bucket: 1})) {
		t.Fatal("LSB round-trip carrier mismatch")
	}
}
