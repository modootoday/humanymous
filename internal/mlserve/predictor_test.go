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
