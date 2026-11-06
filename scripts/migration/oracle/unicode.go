package main

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"unicode"
)

func exportUnicode() {
	type vector struct {
		Input  string `json:"input"`
		Lower  string `json:"lower"`
		Quoted string `json:"quoted"`
	}
	var lower [][2]uint32
	var printable [][2]uint32
	start := rune(-1)
	for character := rune(0); character <= unicode.MaxRune; character++ {
		if mapped := unicode.ToLower(character); mapped != character {
			lower = append(lower, [2]uint32{uint32(character), uint32(mapped)})
		}
		if unicode.IsPrint(character) {
			if start < 0 {
				start = character
			}
		} else if start >= 0 {
			printable = append(printable, [2]uint32{uint32(start), uint32(character - 1)})
			start = -1
		}
	}
	if start >= 0 {
		printable = append(printable, [2]uint32{uint32(start), unicode.MaxRune})
	}
	var controls strings.Builder
	for character := rune(0); character < 32; character++ {
		controls.WriteRune(character)
	}
	var vectors []vector
	for _, input := range []string{"", "Mozilla Chrome \"quoted\" \\", "İ ΟΣ User-Agent", "한국어 🦀", "\u00a0\u0085\u200b\u2028", controls.String(), "\U0001fae0", "\U0001fae9"} {
		vectors = append(vectors, vector{input, strings.ToLower(input), fmt.Sprintf("%q", input)})
	}
	encoder := json.NewEncoder(os.Stdout)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(struct {
		Version   string      `json:"version"`
		Lower     [][2]uint32 `json:"lower"`
		Printable [][2]uint32 `json:"printable"`
		Cases     []vector    `json:"cases"`
	}{unicode.Version, lower, printable, vectors}); err != nil {
		panic(err)
	}
}
