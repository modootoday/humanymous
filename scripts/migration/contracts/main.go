// Command contracts translates the frozen Go report structs into context-grouped
// Rust data contracts. It is migration tooling and is retired with the Go oracle.
package main

import (
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"reflect"
	"regexp"
	"sort"
	"strconv"
	"strings"
)

var contexts = map[string]string{
	"CrossCheck": "scoring", "UAClientHints": "client", "UABrand": "client", "ClientReport": "client",
	"Advanced": "client", "Guard": "client", "Environment": "client", "BehaviorSummary": "behavior",
	"MouseFeatures": "behavior", "KeyFeatures": "behavior", "EventFeatures": "behavior",
	"ScoreResult": "scoring", "Contributor": "scoring", "NetworkReport": "network", "SessionReport": "session",
}
var acronym = regexp.MustCompile(`([A-Z]+)([A-Z][a-z])`)
var camel = regexp.MustCompile(`([a-z0-9])([A-Z])`)

func rustIdent(name string) string {
	if strings.HasPrefix(name, "UA") {
		return "Ua" + strings.TrimPrefix(name, "UA")
	}
	return name
}

func snake(name string) string {
	if name == "SecCHUAPresent" {
		return "sec_ch_ua_present"
	}
	return strings.ToLower(camel.ReplaceAllString(acronym.ReplaceAllString(name, "${1}_${2}"), "${1}_${2}"))
}

func rustType(expr ast.Expr) string {
	switch value := expr.(type) {
	case *ast.Ident:
		switch value.Name {
		case "string":
			return "String"
		case "bool":
			return "bool"
		case "int":
			return "i64"
		case "uint32":
			return "u32"
		case "float64":
			return "f64"
		case "any":
			return "serde_json::Value"
		default:
			return rustIdent(value.Name)
		}
	case *ast.ArrayType:
		if value.Len != nil {
			panic("fixed arrays require explicit review")
		}
		return "Option<Vec<" + rustType(value.Elt) + ">>"
	case *ast.MapType:
		return "Option<std::collections::BTreeMap<" + rustType(value.Key) + ", " + rustType(value.Value) + ">>"
	case *ast.SelectorExpr:
		if id, ok := value.X.(*ast.Ident); ok && id.Name == "time" && value.Sel.Name == "Time" {
			return "Timestamp"
		}
	}
	panic(fmt.Sprintf("unsupported contract type %T", expr))
}

func main() {
	file, err := parser.ParseFile(token.NewFileSet(), "internal/signals/signals.go", nil, parser.ParseComments)
	if err != nil {
		panic(err)
	}
	output := map[string]*strings.Builder{}
	imports := map[string]map[string]bool{}
	for _, declaration := range file.Decls {
		general, ok := declaration.(*ast.GenDecl)
		if !ok {
			continue
		}
		for _, spec := range general.Specs {
			typeSpec, ok := spec.(*ast.TypeSpec)
			if !ok {
				continue
			}
			context, ok := contexts[typeSpec.Name.Name]
			if !ok {
				continue
			}
			structure, ok := typeSpec.Type.(*ast.StructType)
			if !ok {
				panic("expected struct")
			}
			if output[context] == nil {
				output[context] = &strings.Builder{}
				imports[context] = map[string]bool{}
			}
			builder := output[context]
			doc := strings.TrimSpace(general.Doc.Text())
			for _, line := range strings.Split(doc, "\n") {
				fmt.Fprintf(builder, "/// %s\n", line)
			}
			fmt.Fprintf(builder, "#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]\n#[serde(default)]\npub struct %s {\n", rustIdent(typeSpec.Name.Name))
			for _, field := range structure.Fields.List {
				if len(field.Names) != 1 || field.Tag == nil {
					panic("explicit named JSON field required")
				}
				tag, err := strconv.Unquote(field.Tag.Value)
				if err != nil {
					panic(err)
				}
				parts := strings.Split(reflect.StructTag(tag).Get("json"), ",")
				wire := parts[0]
				if wire == "" {
					panic("missing JSON tag")
				}
				typeName := rustType(field.Type)
				ast.Inspect(field.Type, func(node ast.Node) bool {
					if id, ok := node.(*ast.Ident); ok {
						if target, known := contexts[id.Name]; known && target != context {
							imports[context][id.Name] = true
						}
						if id.Name == "Signal" {
							imports[context]["Signal"] = true
						}
					}
					return true
				})
				if typeName == "Timestamp" {
					imports[context]["Timestamp"] = true
				}
				doc := strings.TrimSpace(field.Doc.Text())
				if doc == "" {
					doc = strings.TrimSpace(field.Comment.Text())
				}
				if doc == "" {
					doc = "Wire field `" + wire + "`."
				}
				for _, line := range strings.Split(doc, "\n") {
					fmt.Fprintf(builder, "    /// %s\n", line)
				}
				deserializer := "crate::wire::decode::null_default"
				if strings.HasPrefix(typeName, "Option<Vec<") {
					deserializer = "crate::wire::decode::nullable_vec"
				}
				if strings.HasPrefix(typeName, "Option<std::collections::BTreeMap<") {
					deserializer = "crate::wire::decode::nullable_map"
				}
				fmt.Fprintf(builder, "    #[serde(rename = %q, deserialize_with = %q", wire, deserializer)
				if len(parts) > 1 && parts[1] == "omitempty" {
					skip := ""
					switch {
					case strings.HasPrefix(typeName, "Option<Vec<"):
						skip = "crate::wire::decode::optional_vec_empty"
					case strings.HasPrefix(typeName, "Option<std::collections::BTreeMap<"):
						skip = "crate::wire::decode::optional_map_empty"
					case typeName == "String":
						skip = "String::is_empty"
					default:
						panic("review omission for " + typeName)
					}
					fmt.Fprintf(builder, ", skip_serializing_if = %q", skip)
				}
				fmt.Fprintf(builder, ")]\n    pub %s: %s,\n", snake(field.Names[0].Name), typeName)
			}
			builder.WriteString("}\n\n")
		}
	}
	for context, builder := range output {
		var names []string
		for name := range imports[context] {
			names = append(names, name)
		}
		sort.Strings(names)
		header := "// Translated from the frozen Go wire contract by scripts/migration/contracts.\n"
		for _, name := range names {
			if name == "Signal" {
				header += "use crate::observation::Signal;\n"
			} else {
				header += "use super::" + rustIdent(name) + ";\n"
			}
		}
		if err := os.WriteFile(filepath.Join("crates/signals/src/report", context+".rs"), []byte(header+"\n"+builder.String()), 0644); err != nil {
			panic(err)
		}
	}
}
