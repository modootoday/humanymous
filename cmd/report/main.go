// SPDX-License-Identifier: Apache-2.0

// Command report aggregates test/e2e/results.json into docs/report.html
// (plan/05). Timestamp is passed in so the report package stays deterministic.
package main

import (
	"flag"
	"fmt"
	"log"
	"time"

	"github.com/modootoday/humanymous/internal/report"
)

func main() {
	in := flag.String("in", "test/e2e/results.json", "results.json path")
	out := flag.String("out", "docs/report.html", "output HTML path")
	flag.Parse()

	recs, err := report.Load(*in)
	if err != nil {
		log.Fatalf("load: %v", err)
	}
	sum := report.Aggregate(recs)
	note := "generated " + time.Now().Format("2006-01-02 15:04")
	if err := report.RenderHTML(*out, sum, note); err != nil {
		log.Fatalf("render: %v", err)
	}
	fmt.Printf("wrote %s — botTPR=%.0f%% humanFPR=%.0f%% (%d bot runs, %d human runs)\n",
		*out, sum.BotTPR*100, sum.HumanFPR*100, sum.BotRuns, sum.HumanRuns)
}
