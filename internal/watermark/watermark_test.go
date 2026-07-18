package watermark

import (
	"bytes"
	"encoding/binary"
	"encoding/json"
	"hash/crc32"
	"testing"
	"time"
)

var mk = []byte("master-secret-key")

func payload(id uint32) Payload {
	return Payload{TagID: id, SessionID: "sess-abc", AssetID: "/res/logo.png", Bucket: 20300}
}

// roundTrip embeds a carrier and recovers it for a given MIME.
func roundTrip(t *testing.T, mime string, src []byte) {
	t.Helper()
	p := payload(7)
	out, ch, err := Apply(mk, mime, src, p)
	if err != nil {
		t.Fatalf("[%s] apply: %v", mime, err)
	}
	carrier, ch2, err := Recover(mime, out)
	if err != nil {
		t.Fatalf("[%s] recover (channel=%s): %v", mime, ch, err)
	}
	id, tagBytes, err := SplitCarrier(carrier)
	if err != nil {
		t.Fatalf("[%s] split: %v", mime, err)
	}
	if id != 7 {
		t.Fatalf("[%s] tagID want 7 got %d", mime, id)
	}
	if !Verify(mk, p, tagBytes) {
		t.Fatalf("[%s] verify failed (recover channel=%s)", mime, ch2)
	}
}

func TestRoundTrip_AllFamilies(t *testing.T) {
	roundTrip(t, "application/javascript", []byte("function f(){return 1;}\n"))
	roundTrip(t, "text/css", []byte("body{margin:0}\n"))
	roundTrip(t, "text/html", []byte("<!doctype html><h1>hi</h1>"))
	roundTrip(t, "image/svg+xml", []byte(`<svg xmlns="http://www.w3.org/2000/svg"></svg>`))
	roundTrip(t, "application/json", []byte(`{"a":1,"b":[2,3]}`))
	roundTrip(t, "application/octet-stream", []byte{0x00, 0x01, 0x02, 0x03, 0xff})
	roundTrip(t, "image/png", minimalPNG())
	roundTrip(t, "application/wasm", minimalWASM())
}

func TestSemantic_JSONStaysValid(t *testing.T) {
	src := []byte(`{"user":"kim","roles":["a","b"]}`)
	out, _, err := Apply(mk, "application/json", src, payload(1))
	if err != nil {
		t.Fatal(err)
	}
	var v map[string]any
	if err := json.Unmarshal(out, &v); err != nil {
		t.Fatalf("watermarked JSON no longer parses: %v", err)
	}
	if v["user"] != "kim" {
		t.Fatalf("JSON value changed: %v", v)
	}
}

func TestSemantic_PNGChunksIntact(t *testing.T) {
	out, _, err := Apply(mk, "image/png", minimalPNG(), payload(2))
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.HasPrefix(out, pngSig) {
		t.Fatal("PNG signature lost")
	}
	if findChunk(out, "IHDR") < 0 || findChunk(out, "IEND") < 0 {
		t.Fatal("PNG critical chunks lost")
	}
}

func TestSemantic_WASMStillHasMagic(t *testing.T) {
	out, _, err := Apply(mk, "application/wasm", minimalWASM(), payload(3))
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.HasPrefix(out, wasmMagic) {
		t.Fatal("WASM magic lost")
	}
}

func TestTrace_IdentifiesSessionAndDetectsForgery(t *testing.T) {
	l := NewLedger(time.Hour)
	id := l.Allocate()
	p := Payload{TagID: id, SessionID: "victim-42", AssetID: "/res/report.pdf", Bucket: 20300}
	out, ch, err := Apply(mk, "application/octet-stream", []byte("SECRET DOC"), p)
	if err != nil {
		t.Fatal(err)
	}
	l.Put(Record{Payload: p, MIME: "application/octet-stream", Channel: ch, IssuedAt: time.Unix(1, 0)})

	res, err := l.Trace(mk, "application/octet-stream", out)
	if err != nil {
		t.Fatalf("trace: %v", err)
	}
	if res.SessionID != "victim-42" {
		t.Fatalf("trace session want victim-42 got %s", res.SessionID)
	}
	if res.Forged {
		t.Fatal("legit watermark flagged as forged")
	}

	// A forged carrier (wrong key) must be detected.
	forged, _, _ := Apply([]byte("attacker-key"), "application/octet-stream", []byte("SECRET DOC"), p)
	fres, err := l.Trace(mk, "application/octet-stream", forged)
	if err != nil {
		t.Fatalf("trace forged: %v", err)
	}
	if !fres.Forged {
		t.Fatal("forged watermark not detected")
	}
}

func TestFrameUnframe(t *testing.T) {
	c := []byte{1, 2, 3, 4, 5}
	framed := Frame(c)
	// embed inside noise
	noisy := append([]byte("prefix noise"), framed...)
	noisy = append(noisy, []byte("suffix")...)
	got, err := Unframe(noisy)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(got, c) {
		t.Fatalf("unframe mismatch: %v", got)
	}
}

// --- minimal fixtures ---

func minimalPNG() []byte {
	out := append([]byte(nil), pngSig...)
	ihdr := make([]byte, 13) // width/height/bitdepth... zeros ok for structure test
	out = append(out, buildChunk("IHDR", ihdr)...)
	out = append(out, buildChunk("IEND", nil)...)
	return out
}

func minimalWASM() []byte {
	out := append([]byte(nil), wasmMagic...)
	out = append(out, 0x01, 0x00, 0x00, 0x00) // version 1
	return out
}

// ensure imports used
var _ = binary.BigEndian
var _ = crc32.ChecksumIEEE
