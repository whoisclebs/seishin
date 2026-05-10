use std::any::{Any, TypeId};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fmt;

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeLabel(String);

impl NodeLabel {
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Debug for NodeLabel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("NodeLabel").field(&self.0).finish()
    }
}

impl fmt::Display for NodeLabel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl From<&str> for NodeLabel {
    fn from(label: &str) -> Self {
        Self::new(label)
    }
}

impl From<String> for NodeLabel {
    fn from(label: String) -> Self {
        Self::new(label)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderGraphError {
    MissingNode(NodeLabel),
    DuplicateNode(NodeLabel),
    MissingEdge { from: NodeLabel, to: NodeLabel },
    DuplicateEdge { from: NodeLabel, to: NodeLabel },
    MissingPass(NodeLabel),
    DuplicatePass(NodeLabel),
    Cycle,
}

impl fmt::Display for RenderGraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(label) => write!(formatter, "render graph node `{label}` is missing"),
            Self::DuplicateNode(label) => {
                write!(formatter, "render graph node `{label}` already exists")
            }
            Self::MissingEdge { from, to } => {
                write!(formatter, "render graph edge `{from}` -> `{to}` is missing")
            }
            Self::DuplicateEdge { from, to } => {
                write!(
                    formatter,
                    "render graph edge `{from}` -> `{to}` already exists"
                )
            }
            Self::MissingPass(label) => {
                write!(formatter, "render graph pass `{label}` is missing")
            }
            Self::DuplicatePass(label) => {
                write!(formatter, "render graph pass `{label}` already exists")
            }
            Self::Cycle => formatter.write_str("render graph contains a cycle"),
        }
    }
}

impl Error for RenderGraphError {}

#[derive(Clone, Debug, Default)]
pub struct RenderGraph {
    nodes: BTreeSet<NodeLabel>,
    edges: BTreeSet<(NodeLabel, NodeLabel)>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, label: impl Into<NodeLabel>) -> Result<(), RenderGraphError> {
        let label = label.into();
        if !self.nodes.insert(label.clone()) {
            return Err(RenderGraphError::DuplicateNode(label));
        }

        Ok(())
    }

    pub fn add_node_edge(
        &mut self,
        from: impl Into<NodeLabel>,
        to: impl Into<NodeLabel>,
    ) -> Result<(), RenderGraphError> {
        let from = from.into();
        let to = to.into();
        self.require_node(&from)?;
        self.require_node(&to)?;

        if !self.edges.insert((from.clone(), to.clone())) {
            return Err(RenderGraphError::DuplicateEdge { from, to });
        }

        Ok(())
    }

    pub fn remove_node(&mut self, label: impl Into<NodeLabel>) -> Result<(), RenderGraphError> {
        let label = label.into();
        if !self.nodes.remove(&label) {
            return Err(RenderGraphError::MissingNode(label));
        }

        self.edges
            .retain(|(from, to)| from != &label && to != &label);
        Ok(())
    }

    pub fn remove_node_edge(
        &mut self,
        from: impl Into<NodeLabel>,
        to: impl Into<NodeLabel>,
    ) -> Result<(), RenderGraphError> {
        let from = from.into();
        let to = to.into();
        self.require_node(&from)?;
        self.require_node(&to)?;

        if !self.edges.remove(&(from.clone(), to.clone())) {
            return Err(RenderGraphError::MissingEdge { from, to });
        }

        Ok(())
    }

    pub fn contains(&self, label: impl Into<NodeLabel>) -> bool {
        self.nodes.contains(&label.into())
    }

    pub fn execution_order(&self) -> Result<Vec<NodeLabel>, RenderGraphError> {
        let mut incoming_count = BTreeMap::new();
        let mut outgoing = BTreeMap::new();

        for label in &self.nodes {
            incoming_count.insert(label.clone(), 0usize);
            outgoing.insert(label.clone(), Vec::new());
        }

        for (from, to) in &self.edges {
            self.require_node(from)?;
            self.require_node(to)?;

            *incoming_count
                .get_mut(to)
                .expect("incoming counts are initialized for all graph nodes") += 1;
            outgoing
                .get_mut(from)
                .expect("outgoing lists are initialized for all graph nodes")
                .push(to.clone());
        }

        let mut ready = incoming_count
            .iter()
            .filter_map(|(label, count)| (*count == 0).then_some(label.clone()))
            .collect::<BTreeSet<_>>();
        let mut order = Vec::with_capacity(self.nodes.len());

        while let Some(label) = ready.iter().next().cloned() {
            ready.remove(&label);
            order.push(label.clone());

            let neighbors = outgoing
                .get(&label)
                .expect("outgoing lists are initialized for all graph nodes");
            for neighbor in neighbors {
                let count = incoming_count
                    .get_mut(neighbor)
                    .expect("incoming counts are initialized for all graph nodes");
                *count -= 1;
                if *count == 0 {
                    ready.insert(neighbor.clone());
                }
            }
        }

        if order.len() != self.nodes.len() {
            return Err(RenderGraphError::Cycle);
        }

        Ok(order)
    }

    fn require_node(&self, label: &NodeLabel) -> Result<(), RenderGraphError> {
        if self.nodes.contains(label) {
            Ok(())
        } else {
            Err(RenderGraphError::MissingNode(label.clone()))
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RenderGraphRunner;

impl RenderGraphRunner {
    pub fn new() -> Self {
        Self
    }

    pub fn run(
        &self,
        graph: &RenderGraph,
        mut run_node: impl FnMut(&NodeLabel),
    ) -> Result<(), RenderGraphError> {
        for label in graph.execution_order()? {
            run_node(&label);
        }

        Ok(())
    }

    pub fn run_passes(
        &self,
        graph: &RenderGraph,
        resources: &mut RenderGraphResources,
        passes: &mut RenderGraphPasses,
    ) -> Result<(), RenderGraphError> {
        for label in graph.execution_order()? {
            let pass = passes
                .passes
                .get_mut(&label)
                .ok_or_else(|| RenderGraphError::MissingPass(label.clone()))?;
            pass(RenderGraphContext {
                label: &label,
                resources,
            })?;
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct RenderGraphResources {
    resources: HashMap<TypeId, Box<dyn Any>>,
}

impl RenderGraphResources {
    pub fn insert<T: 'static>(&mut self, resource: T) -> Option<T> {
        self.resources
            .insert(TypeId::of::<T>(), Box::new(resource))
            .and_then(|resource| resource.downcast::<T>().ok())
            .map(|resource| *resource)
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.resources
            .get(&TypeId::of::<T>())
            .and_then(|resource| resource.downcast_ref())
    }

    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut(&TypeId::of::<T>())
            .and_then(|resource| resource.downcast_mut())
    }

    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        self.resources
            .remove(&TypeId::of::<T>())
            .and_then(|resource| resource.downcast::<T>().ok())
            .map(|resource| *resource)
    }

    pub fn contains<T: 'static>(&self) -> bool {
        self.resources.contains_key(&TypeId::of::<T>())
    }
}

impl fmt::Debug for RenderGraphResources {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RenderGraphResources")
            .field("len", &self.resources.len())
            .finish()
    }
}

pub struct RenderGraphContext<'a> {
    label: &'a NodeLabel,
    resources: &'a mut RenderGraphResources,
}

impl<'a> RenderGraphContext<'a> {
    pub fn label(&self) -> &NodeLabel {
        self.label
    }

    pub fn resources(&self) -> &RenderGraphResources {
        self.resources
    }

    pub fn resources_mut(&mut self) -> &mut RenderGraphResources {
        self.resources
    }
}

type RenderGraphPass =
    Box<dyn for<'context> FnMut(RenderGraphContext<'context>) -> Result<(), RenderGraphError>>;

#[derive(Default)]
pub struct RenderGraphPasses {
    passes: BTreeMap<NodeLabel, RenderGraphPass>,
}

impl RenderGraphPasses {
    pub fn add(
        &mut self,
        label: impl Into<NodeLabel>,
        pass: impl for<'context> FnMut(RenderGraphContext<'context>) -> Result<(), RenderGraphError>
            + 'static,
    ) -> Result<(), RenderGraphError> {
        let label = label.into();
        if self.passes.contains_key(&label) {
            return Err(RenderGraphError::DuplicatePass(label));
        }

        self.passes.insert(label, Box::new(pass));
        Ok(())
    }

    pub fn contains(&self, label: impl Into<NodeLabel>) -> bool {
        self.passes.contains_key(&label.into())
    }
}

impl fmt::Debug for RenderGraphPasses {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RenderGraphPasses")
            .field("len", &self.passes.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NodeLabel, RenderGraph, RenderGraphError, RenderGraphPasses, RenderGraphResources,
        RenderGraphRunner,
    };

    #[test]
    fn execution_order_respects_edges_and_is_deterministic() {
        let mut graph = RenderGraph::new();

        graph.add_node("extract").unwrap();
        graph.add_node("prepare").unwrap();
        graph.add_node("draw").unwrap();
        graph.add_node("cleanup").unwrap();
        graph.add_node_edge("extract", "prepare").unwrap();
        graph.add_node_edge("prepare", "draw").unwrap();

        let order = graph.execution_order().unwrap();

        assert_eq!(
            order,
            vec![
                NodeLabel::from("cleanup"),
                NodeLabel::from("extract"),
                NodeLabel::from("prepare"),
                NodeLabel::from("draw"),
            ]
        );
    }

    #[test]
    fn add_node_edge_reports_missing_nodes() {
        let mut graph = RenderGraph::new();
        graph.add_node("extract").unwrap();

        let error = graph.add_node_edge("extract", "prepare").unwrap_err();

        assert_eq!(
            error,
            RenderGraphError::MissingNode(NodeLabel::from("prepare"))
        );
    }

    #[test]
    fn duplicate_nodes_and_edges_are_controlled_errors() {
        let mut graph = RenderGraph::new();
        graph.add_node("extract").unwrap();
        graph.add_node("prepare").unwrap();
        graph.add_node_edge("extract", "prepare").unwrap();

        assert_eq!(
            graph.add_node("extract").unwrap_err(),
            RenderGraphError::DuplicateNode(NodeLabel::from("extract"))
        );
        assert_eq!(
            graph.add_node_edge("extract", "prepare").unwrap_err(),
            RenderGraphError::DuplicateEdge {
                from: NodeLabel::from("extract"),
                to: NodeLabel::from("prepare"),
            }
        );
    }

    #[test]
    fn remove_node_removes_incident_edges_and_contains_tracks_nodes() {
        let mut graph = RenderGraph::new();
        graph.add_node("extract").unwrap();
        graph.add_node("prepare").unwrap();
        graph.add_node_edge("extract", "prepare").unwrap();

        assert!(graph.contains("prepare"));
        graph.remove_node("prepare").unwrap();

        assert!(!graph.contains("prepare"));
        assert_eq!(
            graph.execution_order().unwrap(),
            vec![NodeLabel::from("extract")]
        );
    }

    #[test]
    fn remove_node_edge_reports_missing_edges() {
        let mut graph = RenderGraph::new();
        graph.add_node("extract").unwrap();
        graph.add_node("prepare").unwrap();

        let error = graph.remove_node_edge("extract", "prepare").unwrap_err();

        assert_eq!(
            error,
            RenderGraphError::MissingEdge {
                from: NodeLabel::from("extract"),
                to: NodeLabel::from("prepare"),
            }
        );
    }

    #[test]
    fn execution_order_reports_cycles() {
        let mut graph = RenderGraph::new();
        graph.add_node("a").unwrap();
        graph.add_node("b").unwrap();
        graph.add_node_edge("a", "b").unwrap();
        graph.add_node_edge("b", "a").unwrap();

        assert_eq!(
            graph.execution_order().unwrap_err(),
            RenderGraphError::Cycle
        );
    }

    #[test]
    fn runner_visits_nodes_in_execution_order() {
        let mut graph = RenderGraph::new();
        graph.add_node("extract").unwrap();
        graph.add_node("prepare").unwrap();
        graph.add_node("draw").unwrap();
        graph.add_node_edge("extract", "prepare").unwrap();
        graph.add_node_edge("prepare", "draw").unwrap();

        let mut visited = Vec::new();
        RenderGraphRunner::new()
            .run(&graph, |label| visited.push(label.clone()))
            .unwrap();

        assert_eq!(
            visited,
            vec![
                NodeLabel::from("extract"),
                NodeLabel::from("prepare"),
                NodeLabel::from("draw"),
            ]
        );
    }

    #[test]
    fn runner_executes_registered_passes_with_shared_resources() {
        let mut graph = RenderGraph::new();
        graph.add_node("extract").unwrap();
        graph.add_node("draw").unwrap();
        graph.add_node_edge("extract", "draw").unwrap();
        let mut resources = RenderGraphResources::default();
        resources.insert(Vec::<String>::new());
        let mut passes = RenderGraphPasses::default();

        passes
            .add("extract", |mut context| {
                let label = context.label().to_string();
                context
                    .resources_mut()
                    .get_mut::<Vec<String>>()
                    .unwrap()
                    .push(label);
                Ok(())
            })
            .unwrap();
        passes
            .add("draw", |mut context| {
                let label = context.label().to_string();
                context
                    .resources_mut()
                    .get_mut::<Vec<String>>()
                    .unwrap()
                    .push(label);
                Ok(())
            })
            .unwrap();

        RenderGraphRunner::new()
            .run_passes(&graph, &mut resources, &mut passes)
            .unwrap();

        assert_eq!(
            resources.get::<Vec<String>>().unwrap(),
            &vec!["extract".to_string(), "draw".to_string()]
        );
    }

    #[test]
    fn runner_reports_missing_registered_pass() {
        let mut graph = RenderGraph::new();
        graph.add_node("draw").unwrap();
        let mut resources = RenderGraphResources::default();
        let mut passes = RenderGraphPasses::default();

        let error = RenderGraphRunner::new()
            .run_passes(&graph, &mut resources, &mut passes)
            .unwrap_err();

        assert_eq!(
            error,
            RenderGraphError::MissingPass(NodeLabel::from("draw"))
        );
    }
}
