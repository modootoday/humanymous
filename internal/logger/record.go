package logger

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"math"
	"sort"
	"strconv"
	"strings"
	"time"
	"unicode"
	"unicode/utf8"
)

const (
	maxIdentityBytes = 256
	maxMessageBytes  = 1024
	maxFieldBytes    = 1024
	maxFieldKeyBytes = 128
	maxFields        = 64
)

type record struct {
	SchemaVersion string         `json:"schema_version"`
	Timestamp     string         `json:"ts"`
	Level         string         `json:"level"`
	Service       string         `json:"service"`
	Version       string         `json:"version"`
	Node          string         `json:"node"`
	Component     string         `json:"component"`
	Event         string         `json:"event"`
	Message       string         `json:"message"`
	Sequence      uint64         `json:"sequence"`
	Fields        map[string]any `json:"fields"`
	Truncated     bool           `json:"truncated,omitempty"`
}

func newRecord(at time.Time, level slog.Level, service, version, node, message string) *record {
	message, messageChanged := sanitizeWithFlag(message, maxMessageBytes)
	component, componentChanged := sanitizeWithFlag(service+".runtime", maxIdentityBytes)
	return &record{
		SchemaVersion: SchemaVersion,
		Timestamp:     at.UTC().Format(time.RFC3339Nano),
		Level:         levelName(level),
		Service:       service,
		Version:       version,
		Node:          node,
		Component:     component,
		Event:         "runtime.message",
		Message:       message,
		Fields:        make(map[string]any),
		Truncated:     messageChanged || componentChanged,
	}
}

func levelName(level slog.Level) string {
	switch {
	case level >= slog.LevelError:
		return "error"
	case level >= slog.LevelWarn:
		return "warn"
	case level >= slog.LevelInfo:
		return "info"
	default:
		return "debug"
	}
}

func (r *record) addAttr(prefix string, attr slog.Attr) {
	attr.Value = attr.Value.Resolve()
	key, changed := sanitizeWithFlag(attr.Key, maxFieldKeyBytes)
	key = sanitizeKey(key)
	if prefix != "" {
		key = prefix + "." + key
	}
	if changed || key == "" {
		r.Truncated = true
	}
	if key == "" {
		return
	}
	if attr.Value.Kind() == slog.KindGroup {
		for _, child := range attr.Value.Group() {
			r.addAttr(key, child)
		}
		return
	}

	if prefix == "" && key == "component" {
		value, valueChanged := sanitizeWithFlag(attr.Value.String(), maxIdentityBytes)
		if value != "" {
			r.Component = value
		}
		r.Truncated = r.Truncated || valueChanged
		return
	}
	if prefix == "" && key == "event" {
		value, valueChanged := sanitizeWithFlag(attr.Value.String(), maxIdentityBytes)
		if !validEvent(value) {
			value = "runtime.invalid_event"
			valueChanged = true
		}
		r.Event = value
		r.Truncated = r.Truncated || valueChanged
		return
	}
	if len(r.Fields) >= maxFields {
		if _, exists := r.Fields[key]; !exists {
			r.Truncated = true
			return
		}
	}
	value, valueChanged := safeValue(attr.Value)
	r.Fields[key] = value
	r.Truncated = r.Truncated || valueChanged
}

func safeValue(value slog.Value) (any, bool) {
	switch value.Kind() {
	case slog.KindBool:
		return value.Bool(), false
	case slog.KindDuration:
		return value.Duration().String(), false
	case slog.KindFloat64:
		number := value.Float64()
		if math.IsNaN(number) || math.IsInf(number, 0) {
			return "non_finite", true
		}
		return number, false
	case slog.KindInt64:
		return value.Int64(), false
	case slog.KindString:
		return sanitizeWithFlag(value.String(), maxFieldBytes)
	case slog.KindTime:
		return value.Time().UTC().Format(time.RFC3339Nano), false
	case slog.KindUint64:
		return value.Uint64(), false
	case slog.KindAny:
		// Arbitrary error text, byte slices, structs, and String methods may
		// disclose forbidden request data. Emit only a fixed classification.
		return "unsupported", true
	case slog.KindLogValuer:
		return safeValue(value.Resolve())
	default:
		return "unsupported", true
	}
}

func (r *record) fit() {
	if len(r.jsonLine()) <= MaxRecordBytes && len(r.plainLine()) <= MaxRecordBytes {
		return
	}
	r.Truncated = true
	keys := sortedKeys(r.Fields)
	for i := len(keys) - 1; i >= 0; i-- {
		delete(r.Fields, keys[i])
		if len(r.jsonLine()) <= MaxRecordBytes && len(r.plainLine()) <= MaxRecordBytes {
			return
		}
	}
	for r.Message != "" {
		target := len(r.Message) - 64
		if target < 0 {
			target = 0
		}
		r.Message = truncateUTF8(r.Message, target)
		if len(r.jsonLine()) <= MaxRecordBytes && len(r.plainLine()) <= MaxRecordBytes {
			return
		}
	}
}

func (r *record) jsonLine() []byte {
	encoded, _ := json.Marshal(r)
	return append(encoded, '\n')
}

func (r *record) plainLine() []byte {
	var b strings.Builder
	b.WriteString(r.Timestamp)
	b.WriteByte(' ')
	b.WriteString(strings.ToUpper(r.Level))
	writePlainPair(&b, "service", r.Service)
	writePlainPair(&b, "version", r.Version)
	writePlainPair(&b, "node", r.Node)
	writePlainPair(&b, "component", r.Component)
	writePlainPair(&b, "event", r.Event)
	b.WriteString(" sequence=")
	b.WriteString(strconv.FormatUint(r.Sequence, 10))
	writePlainPair(&b, "message", r.Message)
	for _, key := range sortedKeys(r.Fields) {
		b.WriteString(" field.")
		b.WriteString(key)
		b.WriteByte('=')
		b.WriteString(plainValue(r.Fields[key]))
	}
	if r.Truncated {
		b.WriteString(" truncated=true")
	}
	b.WriteByte('\n')
	return []byte(b.String())
}

func writePlainPair(b *strings.Builder, key, value string) {
	b.WriteByte(' ')
	b.WriteString(key)
	b.WriteByte('=')
	b.WriteString(strconv.Quote(value))
}

func plainValue(value any) string {
	switch typed := value.(type) {
	case string:
		return strconv.Quote(typed)
	case bool:
		return strconv.FormatBool(typed)
	case int64:
		return strconv.FormatInt(typed, 10)
	case uint64:
		return strconv.FormatUint(typed, 10)
	case float64:
		return strconv.FormatFloat(typed, 'g', -1, 64)
	default:
		return strconv.Quote(fmt.Sprint(typed))
	}
}

func sortedKeys(values map[string]any) []string {
	keys := make([]string, 0, len(values))
	for key := range values {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	return keys
}

func sanitizeString(value string, maxBytes int) string {
	clean, _ := sanitizeWithFlag(value, maxBytes)
	return clean
}

func sanitizeWithFlag(value string, maxBytes int) (string, bool) {
	original := value
	value = strings.ToValidUTF8(value, "\uFFFD")
	value = stripANSI(value)
	var b strings.Builder
	b.Grow(len(value))
	for _, current := range value {
		if unicode.IsControl(current) || isDirectionOrSeparator(current) {
			continue
		}
		b.WriteRune(current)
	}
	clean := b.String()
	if len(clean) > maxBytes {
		clean = truncateUTF8(clean, maxBytes)
	}
	return clean, clean != original
}

func truncateUTF8(value string, maxBytes int) string {
	if maxBytes <= 0 {
		return ""
	}
	if len(value) <= maxBytes {
		return value
	}
	value = value[:maxBytes]
	for !utf8.ValidString(value) {
		value = value[:len(value)-1]
	}
	return value
}

func stripANSI(value string) string {
	var b strings.Builder
	for i := 0; i < len(value); {
		if value[i] != 0x1b {
			b.WriteByte(value[i])
			i++
			continue
		}
		i++
		if i >= len(value) {
			break
		}
		switch value[i] {
		case '[':
			i++
			for i < len(value) {
				final := value[i] >= 0x40 && value[i] <= 0x7e
				i++
				if final {
					break
				}
			}
		case ']':
			i++
			for i < len(value) {
				if value[i] == 0x07 {
					i++
					break
				}
				if value[i] == 0x1b && i+1 < len(value) && value[i+1] == '\\' {
					i += 2
					break
				}
				i++
			}
		default:
			i++
		}
	}
	return b.String()
}

func isDirectionOrSeparator(current rune) bool {
	switch {
	case current == '\u061c', current == '\u200e', current == '\u200f':
		return true
	case current >= '\u202a' && current <= '\u202e':
		return true
	case current >= '\u2066' && current <= '\u2069':
		return true
	case current == '\u2028', current == '\u2029', current == '\ufeff':
		return true
	default:
		return false
	}
}

func sanitizeKey(value string) string {
	var b strings.Builder
	for _, current := range strings.ToLower(value) {
		switch {
		case current >= 'a' && current <= 'z':
			b.WriteRune(current)
		case current >= '0' && current <= '9':
			b.WriteRune(current)
		case current == '_', current == '.':
			b.WriteRune(current)
		default:
			b.WriteByte('_')
		}
	}
	return strings.Trim(b.String(), "_.")
}

func validEvent(value string) bool {
	if value == "" || value[0] < 'a' || value[0] > 'z' {
		return false
	}
	lastDot := false
	for _, current := range value {
		switch {
		case current >= 'a' && current <= 'z':
			lastDot = false
		case current >= '0' && current <= '9':
			lastDot = false
		case current == '_':
			lastDot = false
		case current == '.':
			if lastDot {
				return false
			}
			lastDot = true
		default:
			return false
		}
	}
	return !lastDot
}
