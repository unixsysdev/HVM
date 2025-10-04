use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Json(serde_json::Error),
    InvalidFormat(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Json(err) => write!(f, "failed to parse tree specification: {}", err),
            Error::InvalidFormat(msg) => write!(f, "invalid tree specification: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Json(err) => Some(err),
            Error::InvalidFormat(_) => None,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Json(value)
    }
}

#[derive(Deserialize)]
struct RawTreeSpec {
    #[serde(default = "default_alphabet")]
    alphabet: u32,
    #[serde(default)]
    functions: HashMap<String, RawFunctionSpec>,
    tree: RawNode,
}

fn default_alphabet() -> u32 {
    2
}

#[derive(Deserialize)]
struct RawFunctionSpec {
    arity: usize,
    table: Vec<u32>,
}

#[derive(Deserialize, Clone)]
#[serde(tag = "type")]
enum RawNode {
    #[serde(rename = "leaf")]
    Leaf { value: u32 },
    #[serde(rename = "node")]
    Node {
        function: String,
        children: Vec<RawNode>,
    },
}

#[derive(Clone)]
struct FunctionSpec {
    name: String,
    arity: usize,
    table: Vec<u32>,
}

impl FunctionSpec {
    fn apply(&self, args: &[u32], alphabet: u32) -> Result<u32, Error> {
        if args.len() != self.arity {
            return Err(Error::InvalidFormat(format!(
                "function '{}' expected {} arguments, found {}",
                self.name,
                self.arity,
                args.len()
            )));
        }
        let base = alphabet as usize;
        let mut index = 0usize;
        for &value in args {
            if value >= alphabet {
                return Err(Error::InvalidFormat(format!(
                    "value {} is outside the alphabet [0, {})",
                    value, alphabet
                )));
            }
            index = index * base + value as usize;
        }
        let Some(result) = self.table.get(index).copied() else {
            return Err(Error::InvalidFormat(format!(
                "function '{}' table is missing entry {}",
                self.name, index
            )));
        };
        Ok(result)
    }
}

struct Node {
    kind: NodeKind,
}

enum NodeKind {
    Leaf(u32),
    Internal {
        function: usize,
        children: Vec<usize>,
    },
}

pub struct TreeSpec {
    alphabet: u32,
    functions: Vec<FunctionSpec>,
    nodes: Vec<Node>,
    root: usize,
    height: usize,
}

impl TreeSpec {
    pub fn from_json(source: &str) -> Result<Self, Error> {
        let raw: RawTreeSpec = serde_json::from_str(source)?;
        Self::from_raw(raw)
    }

    fn from_raw(raw: RawTreeSpec) -> Result<Self, Error> {
        if raw.functions.is_empty() {
            return Err(Error::InvalidFormat(
                "the specification must declare at least one function".to_string(),
            ));
        }
        if raw.alphabet == 0 {
            return Err(Error::InvalidFormat(
                "the alphabet size must be at least 1".to_string(),
            ));
        }
        let mut function_names: Vec<_> = raw.functions.keys().cloned().collect();
        function_names.sort();
        let mut functions = Vec::with_capacity(function_names.len());
        for name in &function_names {
            let spec = &raw.functions[name];
            let required_len = raw.alphabet.pow(spec.arity as u32) as usize;
            if spec.table.len() != required_len {
                return Err(Error::InvalidFormat(format!(
                    "function '{}' expects table of length {} (alphabet^arity), found {}",
                    name,
                    required_len,
                    spec.table.len()
                )));
            }
            if let Some((position, value)) = spec
                .table
                .iter()
                .enumerate()
                .find(|(_, value)| **value >= raw.alphabet)
            {
                return Err(Error::InvalidFormat(format!(
                    "function '{}' table entry {} outputs {} outside the alphabet [0, {})",
                    name, position, *value, raw.alphabet
                )));
            }
            functions.push(FunctionSpec {
                name: name.clone(),
                arity: spec.arity,
                table: spec.table.clone(),
            });
        }
        let mut index_by_name = HashMap::new();
        for (index, name) in function_names.iter().enumerate() {
            index_by_name.insert(name.clone(), index);
        }
        let mut nodes = Vec::<Node>::new();
        let mut height = 0usize;
        let root = build_nodes(
            &raw.tree,
            raw.alphabet,
            &functions,
            &index_by_name,
            &mut nodes,
            &mut height,
            0,
        )?;
        Ok(TreeSpec {
            alphabet: raw.alphabet,
            functions,
            nodes,
            root,
            height,
        })
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn default_cache_capacity(&self) -> usize {
        if self.nodes.is_empty() {
            return 0;
        }
        let n = self.nodes.len() as f64;
        let h = (self.height.max(1)) as f64;
        let budget = (n * (h.ln() + 1.0)).sqrt().ceil() as usize;
        budget.max(1)
    }

    pub fn evaluate_naive(&self) -> Result<u32, Error> {
        fn eval(spec: &TreeSpec, node_id: usize) -> Result<u32, Error> {
            match &spec.nodes[node_id].kind {
                NodeKind::Leaf(value) => Ok(*value),
                NodeKind::Internal { function, children } => {
                    let mut args = Vec::with_capacity(children.len());
                    for child in children {
                        args.push(eval(spec, *child)?);
                    }
                    spec.functions[*function].apply(&args, spec.alphabet)
                }
            }
        }
        eval(self, self.root)
    }
}

fn build_nodes(
    node: &RawNode,
    alphabet: u32,
    functions: &[FunctionSpec],
    index_by_name: &HashMap<String, usize>,
    nodes: &mut Vec<Node>,
    height: &mut usize,
    depth: usize,
) -> Result<usize, Error> {
    match node {
        RawNode::Leaf { value } => {
            if *value >= alphabet {
                return Err(Error::InvalidFormat(format!(
                    "leaf value {} is outside the alphabet [0, {})",
                    value, alphabet
                )));
            }
            let id = nodes.len();
            nodes.push(Node {
                kind: NodeKind::Leaf(*value),
            });
            *height = (*height).max(depth);
            Ok(id)
        }
        RawNode::Node { function, children } => {
            let Some(&function_index) = index_by_name.get(function) else {
                return Err(Error::InvalidFormat(format!(
                    "function '{}' referenced by a node is undefined",
                    function
                )));
            };
            let spec = &functions[function_index];
            if children.len() != spec.arity {
                return Err(Error::InvalidFormat(format!(
                    "function '{}' expects {} children, found {}",
                    function,
                    spec.arity,
                    children.len()
                )));
            }
            let mut child_ids = Vec::with_capacity(children.len());
            for child in children {
                let child_id = build_nodes(
                    child,
                    alphabet,
                    functions,
                    index_by_name,
                    nodes,
                    height,
                    depth + 1,
                )?;
                child_ids.push(child_id);
            }
            let id = nodes.len();
            nodes.push(Node {
                kind: NodeKind::Internal {
                    function: function_index,
                    children: child_ids,
                },
            });
            *height = (*height).max(depth);
            Ok(id)
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Stats {
    pub peak_cache_entries: usize,
    pub peak_stack_depth: usize,
    pub evictions: usize,
    stack_depth: usize,
}

impl Stats {
    fn enter_frame(&mut self) {
        self.stack_depth += 1;
        if self.stack_depth > self.peak_stack_depth {
            self.peak_stack_depth = self.stack_depth;
        }
    }

    fn leave_frame(&mut self) {
        self.stack_depth = self.stack_depth.saturating_sub(1);
    }

    fn update_cache_usage(&mut self, entries: usize) {
        if entries > self.peak_cache_entries {
            self.peak_cache_entries = entries;
        }
    }
}

struct PebbleCache {
    capacity: usize,
    entries: HashMap<usize, u32>,
    order: VecDeque<usize>,
}

impl PebbleCache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn get(&mut self, key: &usize) -> Option<u32> {
        if let Some(value) = self.entries.get(key).copied() {
            self.touch(key);
            Some(value)
        } else {
            None
        }
    }

    fn touch(&mut self, key: &usize) {
        if let Some(position) = self.order.iter().position(|existing| existing == key) {
            self.order.remove(position);
        }
        self.order.push_back(*key);
    }

    fn insert(&mut self, key: usize, value: u32, stats: &mut Stats) {
        if self.capacity == 0 {
            return;
        }
        if let Some(entry) = self.entries.get_mut(&key) {
            *entry = value;
            self.touch(&key);
            return;
        }
        if self.entries.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
                stats.evictions += 1;
            }
        }
        self.entries.insert(key, value);
        self.order.push_back(key);
    }
}

pub struct Evaluator {
    spec: TreeSpec,
    cache: PebbleCache,
    stats: Stats,
}

impl Evaluator {
    pub fn new(spec: TreeSpec, cache_capacity: Option<usize>) -> Self {
        let default_capacity = spec.default_cache_capacity();
        let capacity = cache_capacity.unwrap_or(default_capacity);
        Self {
            spec,
            cache: PebbleCache::new(capacity),
            stats: Stats::default(),
        }
    }

    pub fn evaluate(&mut self) -> Result<u32, Error> {
        let result = self.eval_node(self.spec.root)?;
        self.stats.update_cache_usage(self.cache.len());
        Ok(result)
    }

    fn eval_node(&mut self, node_id: usize) -> Result<u32, Error> {
        match &self.spec.nodes[node_id].kind {
            NodeKind::Leaf(value) => Ok(*value),
            NodeKind::Internal { function, children } => {
                if let Some(value) = self.cache.get(&node_id) {
                    self.stats.update_cache_usage(self.cache.len());
                    return Ok(value);
                }
                let function_index = *function;
                let child_ids = children.clone();
                self.stats.enter_frame();
                let mut args = Vec::with_capacity(child_ids.len());
                for child in child_ids {
                    match self.eval_node(child) {
                        Ok(value) => args.push(value),
                        Err(err) => {
                            self.stats.leave_frame();
                            return Err(err);
                        }
                    }
                }
                self.stats.leave_frame();
                let value = self.spec.functions[function_index].apply(&args, self.spec.alphabet)?;
                self.cache.insert(node_id, value, &mut self.stats);
                self.stats.update_cache_usage(self.cache.len());
                Ok(value)
            }
        }
    }

    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    pub fn cache_capacity(&self) -> usize {
        self.cache.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> TreeSpec {
        let json = r#"{
            "alphabet": 2,
            "functions": {
                "AND": {"arity": 2, "table": [0,0,0,1]},
                "OR": {"arity": 2, "table": [0,1,1,1]}
            },
            "tree": {
                "type": "node",
                "function": "OR",
                "children": [
                    {"type": "node", "function": "AND", "children": [
                        {"type": "leaf", "value": 1},
                        {"type": "leaf", "value": 0}
                    ]},
                    {"type": "leaf", "value": 1}
                ]
            }
        }"#;
        TreeSpec::from_json(json).expect("valid spec")
    }

    #[test]
    fn evaluator_matches_naive() {
        let spec = sample_spec();
        let naive = spec.evaluate_naive().unwrap();
        let mut evaluator = Evaluator::new(spec, Some(1));
        let result = evaluator.evaluate().unwrap();
        assert_eq!(naive, result);
    }

    #[test]
    fn cache_capacity_respected() {
        let spec = sample_spec();
        let mut evaluator = Evaluator::new(spec, Some(1));
        evaluator.evaluate().unwrap();
        assert!(evaluator.stats().peak_cache_entries <= evaluator.cache_capacity());
    }

    #[test]
    fn rejects_leaf_outside_alphabet() {
        let json = r#"{
            "alphabet": 2,
            "functions": {
                "ID": {"arity": 1, "table": [0, 1]}
            },
            "tree": {
                "type": "node",
                "function": "ID",
                "children": [
                    {"type": "leaf", "value": 2}
                ]
            }
        }"#;
        match TreeSpec::from_json(json) {
            Err(Error::InvalidFormat(_)) => {}
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("leaf outside alphabet should be rejected"),
        }
    }

    #[test]
    fn rejects_arity_mismatch() {
        let json = r#"{
            "alphabet": 2,
            "functions": {
                "AND": {"arity": 2, "table": [0, 0, 0, 1]}
            },
            "tree": {
                "type": "node",
                "function": "AND",
                "children": [
                    {"type": "leaf", "value": 1}
                ]
            }
        }"#;
        match TreeSpec::from_json(json) {
            Err(Error::InvalidFormat(_)) => {}
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("arity mismatch should be rejected"),
        }
    }

    #[test]
    fn rejects_table_outside_alphabet() {
        let json = r#"{
            "alphabet": 2,
            "functions": {
                "NOT": {"arity": 1, "table": [0, 2]}
            },
            "tree": {
                "type": "node",
                "function": "NOT",
                "children": [
                    {"type": "leaf", "value": 1}
                ]
            }
        }"#;
        match TreeSpec::from_json(json) {
            Err(Error::InvalidFormat(_)) => {}
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("table entries outside alphabet should be rejected"),
        }
    }

    #[test]
    fn rejects_zero_alphabet() {
        let json = r#"{
            "alphabet": 0,
            "functions": {
                "ID": {"arity": 1, "table": [0]}
            },
            "tree": {"type": "leaf", "value": 0}
        }"#;
        match TreeSpec::from_json(json) {
            Err(Error::InvalidFormat(_)) => {}
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("zero-sized alphabet should be rejected"),
        }
    }

    #[test]
    fn rejects_undefined_function() {
        let json = r#"{
            "alphabet": 2,
            "functions": {
                "ID": {"arity": 1, "table": [0, 1]}
            },
            "tree": {
                "type": "node",
                "function": "MISSING",
                "children": [
                    {"type": "leaf", "value": 0}
                ]
            }
        }"#;
        match TreeSpec::from_json(json) {
            Err(Error::InvalidFormat(_)) => {}
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("nodes using undefined functions should be rejected"),
        }
    }
}
