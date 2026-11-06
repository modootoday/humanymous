package main

import (
	"encoding/json"
	"os"
	"reflect"
	"time"

	"github.com/modootoday/humanymous/internal/signals"
)

func populate(value reflect.Value) {
	if value.Type() == reflect.TypeOf(time.Time{}) {
		value.Set(reflect.ValueOf(time.Date(2026, 11, 7, 4, 0, 0, 9000, time.FixedZone("KST", 9*3600))))
		return
	}
	switch value.Kind() {
	case reflect.Struct:
		for i := 0; i < value.NumField(); i++ {
			populate(value.Field(i))
		}
	case reflect.String:
		value.SetString("sample")
	case reflect.Bool:
		value.SetBool(true)
	case reflect.Int, reflect.Int64:
		value.SetInt(7)
	case reflect.Uint32:
		value.SetUint(7)
	case reflect.Float64:
		value.SetFloat(7.25)
	case reflect.Slice:
		value.Set(reflect.MakeSlice(value.Type(), 1, 1))
		populate(value.Index(0))
	case reflect.Map:
		value.Set(reflect.MakeMap(value.Type()))
		key := reflect.New(value.Type().Key()).Elem()
		populate(key)
		item := reflect.New(value.Type().Elem()).Elem()
		populate(item)
		value.SetMapIndex(key, item)
	case reflect.Interface:
		value.Set(reflect.ValueOf(map[string]any{"sample": true}))
	default:
		panic("review kind " + value.Kind().String())
	}
}

func nullLeaves(value any) any {
	switch v := value.(type) {
	case map[string]any:
		for key, item := range v {
			v[key] = nullLeaves(item)
		}
		return v
	case []any:
		for i, item := range v {
			v[i] = nullLeaves(item)
		}
		return v
	default:
		return nil
	}
}

func exportReports() {
	type vector struct {
		Kind     string                `json:"kind"`
		Input    json.RawMessage       `json:"input"`
		Expected signals.SessionReport `json:"expected"`
	}
	var populated signals.SessionReport
	populate(reflect.ValueOf(&populated).Elem())
	var vectors []vector
	for _, entry := range []struct {
		kind  string
		value any
	}{{"zero", signals.SessionReport{}}, {"populated", populated}} {
		input, err := json.Marshal(entry.value)
		if err != nil {
			panic(err)
		}
		var expected signals.SessionReport
		if err := json.Unmarshal(input, &expected); err != nil {
			panic(err)
		}
		vectors = append(vectors, vector{entry.kind, input, expected})
	}
	input, _ := json.Marshal(populated)
	var generic any
	if err := json.Unmarshal(input, &generic); err != nil {
		panic(err)
	}
	input, _ = json.Marshal(nullLeaves(generic))
	var expected signals.SessionReport
	if err := json.Unmarshal(input, &expected); err != nil {
		panic(err)
	}
	vectors = append(vectors, vector{"null-leaves", input, expected})
	input = []byte(`{"client":{"signals":[{"id":"l1.navigator.webdriver","value":{"large":9007199254740993,"max":18446744073709551615,"fraction":0.9166666666666667},"expectedHuman":9007199254740993}]}}`)
	expected = signals.SessionReport{}
	if err := json.Unmarshal(input, &expected); err != nil {
		panic(err)
	}
	vectors = append(vectors, vector{"untyped-number-decoding", input, expected})
	encoder := json.NewEncoder(os.Stdout)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(vectors); err != nil {
		panic(err)
	}
}
