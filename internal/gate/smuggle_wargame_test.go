package gate

import (
	"net/http/httptest"
	"testing"
)

func TestSmuggleScanResidualWargame(t *testing.T) {
	cases := []struct {
		name string
		hdr  map[string][]string
		want smuggleReason
	}{
		{"te-empty", map[string][]string{"Transfer-Encoding": {""}}, smuggleBadTE},
		{"te-identity", map[string][]string{"Transfer-Encoding": {"identity"}}, smuggleBadTE},
		{"te-gzip-chunked", map[string][]string{"Transfer-Encoding": {"gzip, chunked"}}, smuggleBadTE},
		{"obs-fold-lf-only", map[string][]string{"X-Foo": {"a\nb"}}, smuggleObsFold},
		{"cl-identical-dup", map[string][]string{"Content-Length": {"5", "5"}}, smuggleDupCL},
	}
	for _, c := range cases {
		r := httptest.NewRequest("POST", "http://p/", nil)
		r.Header = map[string][]string{}
		for k, v := range c.hdr {
			r.Header[k] = v
		}
		if got := smuggleScan(r); got != c.want {
			t.Errorf("%s: got %q want %q", c.name, got, c.want)
		}
	}
}
