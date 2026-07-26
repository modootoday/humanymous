package logger

import (
	"context"
	"log/slog"
	"time"
)

type handler struct {
	runtime *Runtime
	attrs   []slog.Attr
	groups  []string
}

func (h *handler) Enabled(_ context.Context, level slog.Level) bool {
	return !h.runtime.off && level >= h.runtime.level
}

func (h *handler) Handle(_ context.Context, source slog.Record) error {
	if !h.Enabled(context.Background(), source.Level) {
		return nil
	}
	timestamp := source.Time
	if h.runtime.now != nil {
		timestamp = h.runtime.now()
	} else if timestamp.IsZero() {
		timestamp = time.Now()
	}
	rec := newRecord(
		timestamp,
		source.Level,
		h.runtime.service,
		h.runtime.version,
		h.runtime.node,
		source.Message,
	)
	for _, attr := range h.attrs {
		rec.addAttr("", attr)
	}
	prefix := groupPrefix(h.groups)
	source.Attrs(func(attr slog.Attr) bool {
		rec.addAttr(prefix, attr)
		return true
	})

	h.runtime.stateMu.RLock()
	defer h.runtime.stateMu.RUnlock()
	if h.runtime.closed {
		h.runtime.stats.closed.add(source.Level)
		return nil
	}
	select {
	case h.runtime.queue <- queueItem{record: rec}:
	default:
		h.runtime.stats.queueFull.add(source.Level)
	}
	return nil
}

func (h *handler) WithAttrs(attrs []slog.Attr) slog.Handler {
	clone := &handler{
		runtime: h.runtime,
		attrs:   make([]slog.Attr, 0, len(h.attrs)+len(attrs)),
		groups:  append([]string(nil), h.groups...),
	}
	clone.attrs = append(clone.attrs, h.attrs...)
	prefix := groupPrefix(clone.groups)
	for _, attr := range attrs {
		clone.attrs = append(clone.attrs, prefixAttr(prefix, attr))
	}
	return clone
}

func (h *handler) WithGroup(name string) slog.Handler {
	clone := &handler{
		runtime: h.runtime,
		attrs:   append([]slog.Attr(nil), h.attrs...),
		groups:  append([]string(nil), h.groups...),
	}
	name = sanitizeKey(name)
	if name != "" {
		clone.groups = append(clone.groups, name)
	}
	return clone
}

func groupPrefix(groups []string) string {
	var prefix string
	for _, group := range groups {
		if prefix != "" {
			prefix += "."
		}
		prefix += group
	}
	return prefix
}

func prefixAttr(prefix string, attr slog.Attr) slog.Attr {
	if prefix == "" {
		return attr
	}
	attr.Key = prefix + "." + attr.Key
	return attr
}
