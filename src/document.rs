use serde_json::Value;

#[derive(Debug)]
pub struct Node {
    pub key: Option<String>,
    pub pointer: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub depth: usize,
    pub line: usize,
    pub end_line: usize,
    pub comma: bool,
}

pub struct Document {
    pub value: Value,
    pub nodes: Vec<Node>,
    pub lines: usize,
}

impl Document {
    pub fn new(value: Value) -> Self {
        let mut nodes = Vec::new();
        let mut line = 1;
        Self::visit(
            &value,
            None,
            String::new(),
            None,
            0,
            false,
            &mut line,
            &mut nodes,
        );
        Self {
            value,
            nodes,
            lines: line,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn visit(
        value: &Value,
        key: Option<String>,
        pointer: String,
        parent: Option<usize>,
        depth: usize,
        comma: bool,
        line: &mut usize,
        nodes: &mut Vec<Node>,
    ) -> usize {
        let id = nodes.len();
        nodes.push(Node {
            key,
            pointer: pointer.clone(),
            parent,
            children: vec![],
            depth,
            line: *line,
            end_line: *line,
            comma,
        });
        let children: Vec<_> = match value {
            Value::Object(map) => map
                .iter()
                .map(|(k, v)| (Some(k.clone()), k.clone(), v))
                .collect(),
            Value::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, v)| (None, i.to_string(), v))
                .collect(),
            _ => vec![],
        };
        let len = children.len();
        for (i, (key, token, child)) in children.into_iter().enumerate() {
            *line += 1;
            let escaped = token.replace('~', "~0").replace('/', "~1");
            let child_id = Self::visit(
                child,
                key,
                format!("{pointer}/{escaped}"),
                Some(id),
                depth + 1,
                i + 1 < len,
                line,
                nodes,
            );
            nodes[id].children.push(child_id);
        }
        if len > 0 {
            *line += 1;
        }
        nodes[id].end_line = *line;
        id
    }

    pub fn value(&self, id: usize) -> &Value {
        self.value
            .pointer(&self.nodes[id].pointer)
            .expect("indexed JSON pointer")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeRow {
    pub id: usize,
    pub closing: bool,
}

pub fn visible_rows(doc: &Document, collapsed: &std::collections::HashSet<usize>) -> Vec<TreeRow> {
    fn walk(
        doc: &Document,
        collapsed: &std::collections::HashSet<usize>,
        id: usize,
        rows: &mut Vec<TreeRow>,
    ) {
        rows.push(TreeRow { id, closing: false });
        if !collapsed.contains(&id) && !doc.nodes[id].children.is_empty() {
            for child in &doc.nodes[id].children {
                walk(doc, collapsed, *child, rows);
            }
            rows.push(TreeRow { id, closing: true });
        }
    }
    let mut rows = vec![];
    walk(doc, collapsed, 0, &mut rows);
    rows
}

pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub node_ids: Vec<usize>,
}

impl TableData {
    pub fn new(doc: &Document, id: usize) -> Self {
        let node = &doc.nodes[id];
        match doc.value(id) {
            Value::Array(items) if !items.is_empty() && items.iter().all(Value::is_object) => {
                let mut keys = Vec::new();
                for item in items {
                    for key in item.as_object().unwrap().keys() {
                        if !keys.contains(key) {
                            keys.push(key.clone());
                        }
                    }
                }
                // A value column makes arrays of empty objects inspectable too.
                if keys.is_empty() {
                    return Self::values(doc, id, "Index");
                }
                let rows = items
                    .iter()
                    .map(|item| {
                        keys.iter()
                            .map(|key| {
                                item.get(key)
                                    .map(Value::to_string)
                                    .unwrap_or_else(|| "—".into())
                            })
                            .collect()
                    })
                    .collect();
                Self {
                    headers: keys,
                    rows,
                    node_ids: node.children.clone(),
                }
            }
            Value::Object(_) => Self::values(doc, id, "Key"),
            Value::Array(_) => Self::values(doc, id, "Index"),
            value => Self {
                headers: vec!["Value".into()],
                rows: vec![vec![value.to_string()]],
                node_ids: vec![id],
            },
        }
    }

    fn values(doc: &Document, id: usize, label: &str) -> Self {
        let node_ids = doc.nodes[id].children.clone();
        let rows = node_ids
            .iter()
            .enumerate()
            .map(|(index, child)| {
                let key = doc.nodes[*child]
                    .key
                    .as_ref()
                    .map(|s| serde_json::to_string(s).unwrap())
                    .unwrap_or_else(|| index.to_string());
                vec![key, doc.value(*child).to_string()]
            })
            .collect();
        Self {
            headers: vec![label.into(), "Value".into()],
            rows,
            node_ids,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    #[test]
    fn stable_lines_depth_and_pointer_escaping() {
        let doc = Document::new(json!({"a/b~": [1, {"x": true}], "empty": {}}));
        assert_eq!(
            doc.lines,
            serde_json::to_string_pretty(&doc.value)
                .unwrap()
                .lines()
                .count()
        );
        let nested = doc
            .nodes
            .iter()
            .position(|n| n.pointer == "/a~1b~0/1/x")
            .unwrap();
        assert_eq!(doc.value(nested), &json!(true));
        assert_eq!(doc.nodes[nested].depth, 3);
        let child = doc.nodes[0].children[0];
        let rows = visible_rows(&doc, &HashSet::from([child]));
        assert!(!rows.iter().any(|r| r.id == nested));
        let sibling = *doc.nodes[0].children.last().unwrap();
        assert_eq!(doc.nodes[sibling].line, 8);
    }

    #[test]
    fn tables_union_columns_and_keep_nested_json() {
        let doc = Document::new(json!([{"a": {"x": [1,2]}}, {"b": null, "a": "ok"}]));
        let table = TableData::new(&doc, 0);
        assert_eq!(table.headers, ["a", "b"]);
        assert_eq!(table.rows[0], ["{\"x\":[1,2]}", "—"]);
        assert_eq!(table.rows[1], ["\"ok\"", "null"]);
    }

    #[test]
    fn table_handles_mixed_empty_and_scalar_values() {
        for input in ["[]", "{}", "null", "[{},{}]", "[1,{},[true]]"] {
            let doc = Document::new(serde_json::from_str(input).unwrap());
            let table = TableData::new(&doc, 0);
            assert_eq!(table.rows.len(), table.node_ids.len());
            assert!(table.rows.iter().all(|r| r.len() == table.headers.len()));
        }
    }
}
