package mlserve

import (
	"testing"

	"github.com/modootoday/humanymous/internal/behavior"
)

// fakePredictor returns a fixed p(bot) and a caller-supplied schema hash.
type fakePredictor struct {
	p      float32
	schema string
}

func (f fakePredictor) Predict(behavior.FeatureVector) Prediction { return Prediction{PBot: f.p} }
func (f fakePredictor) SchemaHash() string                        { return f.schema }
func (f fakePredictor) BundleVersion() string                     { return "test" }

// emptyFV is a zero-length feature vector; the seam's abstain/guard logic is independent of
// feature content, so this is sufficient to exercise it.
var emptyFV = behavior.FeatureVector{}

func TestDefault_Abstains(t *testing.T) {
	Set(nil) // reset to AbstainPredictor
	if got := Score(emptyFV); !got.Abstain {
		t.Fatalf("default predictor must abstain, got %+v", got)
	}
}

func TestScore_MatchingSchema(t *testing.T) {
	Set(fakePredictor{p: 0.8, schema: behavior.SchemaHash()})
	defer Set(nil)
	got := Score(emptyFV)
	if got.Abstain {
		t.Fatal("a predictor with a matching schema must NOT abstain")
	}
	if got.PBot != 0.8 {
		t.Fatalf("want p=0.8 got %v", got.PBot)
	}
}

func TestScore_StaleSchemaAbstains(t *testing.T) {
	// A model built against a different feature schema must fail closed to heuristics.
	Set(fakePredictor{p: 0.99, schema: "deadbeefdeadbeefdeadbeef"})
	defer Set(nil)
	if got := Score(emptyFV); !got.Abstain {
		t.Fatalf("stale-schema predictor must abstain (fail closed), got %+v", got)
	}
}

type captureShadowObs struct {
	active, shadow Prediction
	n              int
}

func (c *captureShadowObs) ObserveShadow(a, s Prediction) { c.active, c.shadow = a, s; c.n++ }

func TestScore_ShadowObservedNeverServed(t *testing.T) {
	Set(fakePredictor{p: 0.3, schema: behavior.SchemaHash()})
	SetShadow(fakePredictor{p: 0.9, schema: behavior.SchemaHash()})
	obs := &captureShadowObs{}
	SetShadowObserver(obs)
	defer func() { Set(nil); SetShadow(nil); SetShadowObserver(nil) }()

	got := Score(emptyFV)
	if got.Abstain || got.PBot != 0.3 {
		t.Fatalf("Score must return the ACTIVE prediction (0.3), never the shadow, got %+v", got)
	}
	if obs.n != 1 || obs.active.PBot != 0.3 || obs.shadow.PBot != 0.9 {
		t.Fatalf("shadow observer must see (active=0.3, shadow=0.9), got n=%d %+v / %+v", obs.n, obs.active, obs.shadow)
	}
}

func TestScore_ShadowStaleSchemaSkipped(t *testing.T) {
	Set(fakePredictor{p: 0.3, schema: behavior.SchemaHash()})
	SetShadow(fakePredictor{p: 0.9, schema: "stale-schema-hash"})
	obs := &captureShadowObs{}
	SetShadowObserver(obs)
	defer func() { Set(nil); SetShadow(nil); SetShadowObserver(nil) }()

	if got := Score(emptyFV); got.PBot != 0.3 {
		t.Fatalf("active still served, got %+v", got)
	}
	if obs.n != 0 {
		t.Fatal("a stale-schema shadow must be skipped, never observed")
	}
}
