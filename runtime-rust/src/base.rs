use yarrow_validator::errors::*;
use yarrow_validator::ErrorKind::{PrivateError, PublicError};

extern crate yarrow_validator;

use yarrow_validator::{proto, base};
use yarrow_validator::utilities::{graph as yarrow_graph, serial};



use std::collections::{HashMap, HashSet};
use std::vec::Vec;

use itertools::Itertools;

use crate::components;

use yarrow_validator::base::{get_input_properties, Value};

pub type NodeArguments<'a> = HashMap<String, &'a Value>;

pub fn execute_graph(analysis: &proto::Analysis,
                     release: &proto::Release,
                     dataset: &proto::Dataset) -> Result<proto::Release> {
    let node_ids_release: HashSet<u32> = yarrow_graph::get_release_nodes(&analysis)?;

    // stack for storing which nodes to evaluate next
    let mut traversal = Vec::new();
    traversal.extend(yarrow_graph::get_sinks(&analysis).into_iter());

    let mut evaluations = serial::parse_release(release)?;

    let mut graph: HashMap<u32, proto::Component> = analysis.computation_graph.to_owned().unwrap().value;

    let mut graph_properties: HashMap<u32, proto::Properties> = HashMap::new();
    let mut maximum_id = graph.keys()
        .fold1(std::cmp::max)
        .map(|x| x.clone())
        .unwrap_or(0);

    // track node parents
    let mut parents = HashMap::<u32, HashSet<u32>>::new();
    graph.iter().for_each(|(node_id, component)| {
        component.arguments.values().for_each(|source_node_id| {
            parents.entry(*source_node_id).or_insert_with(HashSet::<u32>::new).insert(*node_id);
        })
    });

    while !traversal.is_empty() {
        let node_id: u32 = *traversal.last().unwrap();
        let component: &proto::Component = graph.get(&node_id).unwrap();
        let arguments = component.to_owned().arguments;

        // discover if any dependencies remain uncomputed
        let mut evaluable = true;
        for source_node_id in arguments.values() {
            if !evaluations.contains_key(&source_node_id) {
                evaluable = false;
                traversal.push(*source_node_id);
                break;
            }
        }

        if !evaluable {
            continue;
        }

        let node_properties: HashMap<String, proto::Properties> = get_input_properties(&component, &graph_properties)?;
        let public_arguments = node_properties.iter()
            .filter(|(_k, v)| v.releasable)
            .map(|(k, _v)| (k.clone(), evaluations
                .get(component.arguments.get(k).unwrap()).unwrap().clone()))
            .collect::<HashMap<String, Value>>();

        // all arguments have been computed, attempt to expand the current node
        let expansion: proto::response_expand_component::ExpandedComponent = yarrow_validator::base::expand_component(
            &analysis.privacy_definition.to_owned().unwrap(),
            &component,
            &node_properties,
            &public_arguments,
            node_id,
            maximum_id
        )?;

        graph_properties.insert(node_id, expansion.properties.unwrap());
        graph.extend(expansion.computation_graph.unwrap().value);

        if maximum_id != expansion.maximum_id {
            maximum_id = expansion.maximum_id;
            continue
        }

        traversal.pop();

        let evaluation = execute_component(
            &graph.get(&node_id).unwrap(), &evaluations, &dataset)?;

        evaluations.insert(node_id, evaluation);

        // remove references to parent node, and if empty and private
        for argument_node_id in arguments.values() {
            let tempval = parents.get_mut(argument_node_id).unwrap();
            tempval.remove(&node_id);
            if parents.get(argument_node_id).unwrap().len() == 0 {
                if !node_ids_release.contains(argument_node_id) {
                    evaluations.remove(argument_node_id);
                    // parents.remove(argument_node_id); // optional
                }
            }
        }
    }
    serial::serialize_release(&evaluations)
}

pub fn execute_component(component: &proto::Component,
                         evaluations: &base::Release,
                         dataset: &proto::Dataset) -> Result<Value> {

    println!("executing component:");
    println!("{:?}", component);

    let mut arguments = NodeArguments::new();
    component.arguments.iter().for_each(|(field_id, field)| {
        let evaluation = evaluations.get(&field).unwrap();
        arguments.insert(field_id.to_owned(), evaluation);
    });

    println!("arguments:");
    println!("{:?}", arguments);

    use proto::component::Value as Value;
    match component.to_owned().value.unwrap() {
        Value::Materialize(x) => components::component_materialize(&x, &dataset),
        Value::Count(x) => components::component_count(&x, &arguments),
        Value::Clamp(x) => components::component_clamp(&x, &arguments),
        Value::Impute(x) => components::component_impute(&x, &arguments),
        Value::Index(x) => components::component_index(&x, &arguments),
        Value::Kthrawsamplemoment(x) => components::component_kth_raw_sample_moment(&x, &arguments),
        Value::Resize(x) => components::component_resize(&x, &arguments),
        Value::Literal(x) => components::component_literal(&x),
        Value::Datasource(x) => components::component_datasource(&x, &dataset, &arguments),
        Value::Add(x) => components::component_add(&x, &arguments),
        Value::Subtract(x) => components::component_subtract(&x, &arguments),
        Value::Divide(x) => components::component_divide(&x, &arguments),
        Value::Multiply(x) => components::component_multiply(&x, &arguments),
        Value::Power(x) => components::component_power(&x, &arguments),
        Value::Negate(x) => components::component_negate(&x, &arguments),
        Value::Bin(x) => components::component_bin(&x, &arguments),
        Value::Rowmin(x) => components::component_row_wise_min(&x, &arguments),
        Value::Rowmax(x) => components::component_row_wise_max(&x, &arguments),
        // Value::Count(x) => components::component_count(&x, &arguments),
        // Value::Histogram(x) => components::component_histogram(&x, &arguments),
        Value::Mean(x) => components::component_mean(&x, &arguments),
        Value::Median(x) => components::component_median(&x, &arguments),
        Value::Sum(x) => components::component_sum(&x, &arguments),
        Value::Variance(x) => components::component_variance(&x, &arguments),
        Value::LaplaceMechanism(x) => components::component_laplace_mechanism(&x, &arguments),
        Value::GaussianMechanism(x) => components::component_gaussian_mechanism(&x, &arguments),
        Value::SimpleGeometricMechanism(x) => components::component_simple_geometric_mechanism(&x, &arguments),
        variant => Err(format!("Component type not implemented: {:?}", variant).into())
    }
}
