use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Attribute aliases: XML attr name → JSON key name
fn alias_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("class", "className");
    m.insert("resource-id", "resourceId");
    m.insert("content-desc", "description");
    m.insert("long-clickable", "longClickable");
    m.insert("bounds", "rect");
    m
}

/// Parse bounds string "[x1,y1][x2,y2]" → {x, y, width, height}
fn parse_bounds(text: &str) -> Option<Value> {
    // Regex-like parsing
    let nums: Vec<i64> = text
        .replace('[', "")
        .replace(']', ",")
        .split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    if nums.len() == 4 {
        let (lx, ly, rx, ry) = (nums[0], nums[1], nums[2], nums[3]);
        Some(serde_json::json!({
            "x": lx,
            "y": ly,
            "width": rx - lx,
            "height": ry - ly,
        }))
    } else {
        None
    }
}

fn str2bool(v: &str) -> Value {
    Value::Bool(matches!(v.to_lowercase().as_str(), "yes" | "true" | "t" | "1"))
}

/// Parse a single XML element's attributes into a JSON object.
fn parse_uiautomator_attrs(attrs: &[(String, String)]) -> Map<String, Value> {
    let aliases = alias_map();
    let bool_fields = [
        "checkable",
        "scrollable",
        "focused",
        "clickable",
        "selected",
        "longClickable",
        "focusable",
        "password",
        "enabled",
    ];

    let mut ks = Map::new();

    for (key, value) in attrs {
        let json_key = aliases
            .get(key.as_str())
            .copied()
            .unwrap_or(key.as_str());

        if json_key == "rect" {
            if let Some(bounds) = parse_bounds(value) {
                ks.insert("rect".to_string(), bounds);
            }
        } else if json_key == "index" {
            if let Ok(n) = value.parse::<i64>() {
                ks.insert(json_key.to_string(), Value::Number(n.into()));
            }
        } else if bool_fields.contains(&json_key) {
            ks.insert(json_key.to_string(), str2bool(value));
        } else {
            ks.insert(json_key.to_string(), Value::String(value.clone()));
        }
    }

    ks
}

/// Convert Android UIAutomator XML hierarchy to JSON tree.
/// Mimics the Python `get_android_hierarchy()` function.
pub fn xml_to_json(xml_str: &str) -> Result<Value, String> {
    let mut reader = Reader::from_str(xml_str);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<Value> = Vec::new();
    let mut root: Option<Value> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let attrs: Vec<(String, String)> = e
                    .attributes()
                    .filter_map(|a| a.ok())
                    .map(|a| {
                        let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                        let val = a.unescape_value().unwrap_or_default().to_string();
                        (key, val)
                    })
                    .collect();

                let mut node = Value::Object(parse_uiautomator_attrs(&attrs));
                let obj = node.as_object_mut().unwrap();
                obj.insert("id".to_string(), Value::String(uuid::Uuid::new_v4().to_string()));
                obj.insert("children".to_string(), Value::Array(Vec::new()));

                stack.push(node);
            }
            Ok(Event::Empty(ref e)) => {
                // Self-closing tag, treat as leaf node
                let attrs: Vec<(String, String)> = e
                    .attributes()
                    .filter_map(|a| a.ok())
                    .map(|a| {
                        let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                        let val = a.unescape_value().unwrap_or_default().to_string();
                        (key, val)
                    })
                    .collect();

                let mut node = Value::Object(parse_uiautomator_attrs(&attrs));
                let obj = node.as_object_mut().unwrap();
                obj.insert("id".to_string(), Value::String(uuid::Uuid::new_v4().to_string()));

                if let Some(parent) = stack.last_mut() {
                    if let Some(children) = parent.get_mut("children").and_then(|c| c.as_array_mut())
                    {
                        children.push(node);
                    }
                } else {
                    root = Some(node);
                }
            }
            Ok(Event::End(_)) => {
                if let Some(node) = stack.pop() {
                    if let Some(parent) = stack.last_mut() {
                        if let Some(children) =
                            parent.get_mut("children").and_then(|c| c.as_array_mut())
                        {
                            children.push(node);
                        }
                    } else {
                        root = Some(node);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
    }

    root.ok_or_else(|| "Empty XML hierarchy".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bounds_normal() {
        let result = parse_bounds("[0,0][100,200]").unwrap();
        assert_eq!(result["x"], 0);
        assert_eq!(result["y"], 0);
        assert_eq!(result["width"], 100);
        assert_eq!(result["height"], 200);
    }

    #[test]
    fn test_parse_bounds_offset() {
        let result = parse_bounds("[50,100][200,300]").unwrap();
        assert_eq!(result["x"], 50);
        assert_eq!(result["y"], 100);
        assert_eq!(result["width"], 150);
        assert_eq!(result["height"], 200);
    }

    #[test]
    fn test_parse_bounds_invalid() {
        assert!(parse_bounds("invalid").is_none());
        assert!(parse_bounds("").is_none());
        assert!(parse_bounds("[0,0]").is_none());
    }

    #[test]
    fn test_str2bool_variants() {
        assert_eq!(str2bool("true"), Value::Bool(true));
        assert_eq!(str2bool("yes"), Value::Bool(true));
        assert_eq!(str2bool("1"), Value::Bool(true));
        assert_eq!(str2bool("t"), Value::Bool(true));
        assert_eq!(str2bool("TRUE"), Value::Bool(true));
        assert_eq!(str2bool("false"), Value::Bool(false));
        assert_eq!(str2bool("no"), Value::Bool(false));
        assert_eq!(str2bool("0"), Value::Bool(false));
        assert_eq!(str2bool("random"), Value::Bool(false));
    }

    #[test]
    fn test_xml_to_json_simple() {
        let xml = r#"<node class="android.widget.Button" text="OK" />"#;
        let result = xml_to_json(xml).unwrap();
        assert_eq!(result["className"], "android.widget.Button");
        assert_eq!(result["text"], "OK");
        assert!(result.get("id").is_some());
    }

    #[test]
    fn test_xml_to_json_nested() {
        let xml = r#"<node class="android.widget.FrameLayout">
            <node class="android.widget.Button" text="OK" />
        </node>"#;
        let result = xml_to_json(xml).unwrap();
        assert_eq!(result["className"], "android.widget.FrameLayout");
        let children = result["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["className"], "android.widget.Button");
        assert_eq!(children[0]["text"], "OK");
    }

    #[test]
    fn test_xml_to_json_attr_aliases() {
        let xml = r#"<node class="android.view.View" resource-id="com.app:id/btn" content-desc="Button description" long-clickable="true" bounds="[0,0][100,200]" />"#;
        let result = xml_to_json(xml).unwrap();
        // class → className
        assert_eq!(result["className"], "android.view.View");
        // resource-id → resourceId
        assert_eq!(result["resourceId"], "com.app:id/btn");
        // content-desc → description
        assert_eq!(result["description"], "Button description");
        // long-clickable → longClickable (bool)
        assert_eq!(result["longClickable"], true);
        // bounds → rect
        assert_eq!(result["rect"]["x"], 0);
        assert_eq!(result["rect"]["width"], 100);
        assert_eq!(result["rect"]["height"], 200);
    }
}
