package mltrain

import (
	"github.com/modootoday/humanymous/internal/behavior"
	"github.com/modootoday/humanymous/internal/signals"
)

// Record is the canonical on-disk JSONL schema for a labeled behavioral trace — the single
// interchange format between the live trace sink (cmd/server, oracle-confirmed sessions) and the
// offline trainers (cmd/ml-train, cmd/mlcorrect-train). It carries the RAW behavioral summary, not
// extracted features, so the feature transform stays one source of truth applied at train time
// (train/serve parity). Defining it here keeps the writer and readers from drifting apart.
type Record struct {
	Label    int                     `json:"label"` // 1 = bot, 0 = human
	Behavior signals.BehaviorSummary `json:"behavior"`
	TS       int64                   `json:"ts,omitempty"`     // event time (unix) for the TESSERACT time-split
	Cohort   string                  `json:"cohort,omitempty"` // accessibility cohort for the per-cohort human-FP gate
	Source   string                  `json:"source,omitempty"` // provenance: "pass" | "catalog" | "challenge" | ...
}

// Sample turns a raw record into the in-memory training sample (feature vector + label + metadata),
// applying the shared feature transform.
func (r Record) Sample() Sample {
	return Sample{
		X:      behavior.Extract(r.Behavior),
		Y:      float32(r.Label),
		Human:  r.Label == 0,
		TS:     r.TS,
		Cohort: r.Cohort,
	}
}
