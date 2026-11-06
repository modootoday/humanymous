package main

import (
	"encoding/json"
	"os"

	"github.com/modootoday/humanymous/internal/behavior"
	"github.com/modootoday/humanymous/internal/mlserve"
)

func exportMLP() {
	type vector struct {
		Features []float32          `json:"features"`
		Expected mlserve.Prediction `json:"expected"`
	}
	type model struct {
		Bundle  json.RawMessage `json:"bundle"`
		Vectors []vector        `json:"vectors"`
	}
	var models []model
	for variant := 0; variant < 5; variant++ {
		dim := behavior.FeatureDim
		bundle := mlserve.MLPBundle{Version: "oracle", SchemaHash: behavior.SchemaHash(), FeatureDim: dim, Hidden: 2, Mean: make([]float32, dim), Std: make([]float32, dim), W1: make([][]float32, 2), B1: []float32{0.2, -0.3}, W2: []float32{0.7, -0.4}, B2: float32(variant) - 1.5, CalA: 1.1, CalB: -0.2}
		for i := 0; i < dim; i++ {
			bundle.Mean[i] = float32(i%3) / 10
			bundle.Std[i] = float32(i % 4)
			if variant == 3 {
				bundle.Std[i] = -bundle.Std[i]
			}
		}
		for hidden := 0; hidden < 2; hidden++ {
			bundle.W1[hidden] = make([]float32, dim)
			for i := 0; i < dim; i++ {
				bundle.W1[hidden][i] = float32((i+hidden)%5-2) / 10
			}
		}
		raw, err := json.Marshal(bundle)
		if err != nil {
			panic(err)
		}
		if variant == 4 {
			var fields map[string]json.RawMessage
			if err := json.Unmarshal(raw, &fields); err != nil {
				panic(err)
			}
			for _, name := range []string{"version", "b2", "calB"} {
				fields[name] = json.RawMessage("null")
			}
			for _, name := range []string{"mean", "std", "b1", "w2"} {
				var values []json.RawMessage
				if err := json.Unmarshal(fields[name], &values); err != nil {
					panic(err)
				}
				values[0] = json.RawMessage("null")
				fields[name], err = json.Marshal(values)
				if err != nil {
					panic(err)
				}
			}
			var matrix [][]json.RawMessage
			if err := json.Unmarshal(fields["w1"], &matrix); err != nil {
				panic(err)
			}
			matrix[0][0] = json.RawMessage("null")
			fields["w1"], err = json.Marshal(matrix)
			if err != nil {
				panic(err)
			}
			raw, err = json.Marshal(fields)
			if err != nil {
				panic(err)
			}
		}
		file, err := os.CreateTemp("", "humanymous-model-oracle-*.json")
		if err != nil {
			panic(err)
		}
		if _, err := file.Write(raw); err != nil {
			panic(err)
		}
		if err := file.Close(); err != nil {
			panic(err)
		}
		predictor, err := mlserve.LoadMLP(file.Name())
		os.Remove(file.Name())
		if err != nil {
			panic(err)
		}
		entry := model{Bundle: raw}
		for sample := -32; sample <= 32; sample++ {
			features := make([]float32, dim)
			for i := range features {
				features[i] = float32(sample)/10 + float32(i)/100
			}
			entry.Vectors = append(entry.Vectors, vector{features, predictor.Predict(features)})
		}
		for _, length := range []int{0, dim - 1, dim + 1} {
			features := make([]float32, length)
			entry.Vectors = append(entry.Vectors, vector{features, predictor.Predict(features)})
		}
		models = append(models, entry)
	}
	encoder := json.NewEncoder(os.Stdout)
	if err := encoder.Encode(models); err != nil {
		panic(err)
	}
}
