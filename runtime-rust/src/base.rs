// INTERNAL IMPORTS
extern crate yarrow_validator;
use yarrow_validator::{
    yarrow,
    base::get_sinks,
    base::get_release_nodes
};
use crate::components;

// STANDARD LIB IMPORTS
use std::collections::{HashMap, HashSet};
use std::vec::Vec;

// EXTERNAL IMPORTS
use ndarray::prelude::*;

// equivalent to proto ArrayNd
#[derive(Debug)]
pub enum FieldEvaluation {
    Bytes(ArrayD<u8>), // bytes::Bytes BROKEN: only one byte is stored
    Bool(ArrayD<bool>),
    I64(ArrayD<i64>),
    F64(ArrayD<f64>),
    Str(ArrayD<String>),
}

// equivalent to proto ReleaseNode
pub type NodeEvaluation = HashMap<String, FieldEvaluation>;
// equivalent to proto Release
pub type GraphEvaluation = HashMap<u32, NodeEvaluation>;

// arguments to a node prior to evaluation
pub type NodeArguments<'a> = HashMap<String, &'a FieldEvaluation>;

// implemented on each yarrow component variant
trait Evaluable {
    fn evaluate(&self, arguments: &NodeArguments) -> NodeEvaluation;
}

pub fn compute_release(
    analysis: &yarrow::Analysis,
    release: &yarrow::Release,
    dataset: &yarrow::Dataset
) -> yarrow::Release {

    let node_ids_release: HashSet<u32> = get_release_nodes(&analysis);

    // stack for storing which nodes to evaluate next
    let mut traversal = Vec::new();
    traversal.extend(get_sinks(&analysis).into_iter());

    let mut evaluations = release_to_evaluations(release);
    let graph: &HashMap<u32, yarrow::Component> = &analysis.graph;

    // track node parents
    let mut parents = HashMap::<u32, HashSet<u32>>::new();
    graph.iter().for_each(|(node_id, component)| {
        component.arguments.values().for_each(|field| {
            let argument_node_id = &field.source_node_id;
            parents.entry(*argument_node_id).or_insert_with(HashSet::<u32>::new).insert(*node_id);
        })
    });

    while !traversal.is_empty() {
        let node_id: u32 = *traversal.last().unwrap();
        let component = graph.get(&node_id).unwrap();
        let arguments = component.to_owned().arguments;

        // discover if any dependencies remain uncomputed
        let mut evaluable = true;
        for field in arguments.values() {
            if !evaluations.contains_key(&field.source_node_id) {
                evaluable = false;
                traversal.push(field.source_node_id);
                break;
            }
        }

        // check if all arguments are available
        if evaluable {
            traversal.pop();

            evaluations.insert(node_id, execute_component(
                &graph.get(&node_id).unwrap(), &evaluations, &dataset));

            // remove references to parent node, and if empty and private
            for argument in arguments.values() {
                let argument_node_id = &(argument.source_node_id);
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
    }
    evaluations_to_release(&evaluations)
}

pub fn execute_component(
    component: &yarrow::Component,
    evaluations: &GraphEvaluation,
    dataset: &yarrow::Dataset
) -> NodeEvaluation {
    let arguments = get_arguments(&component, &evaluations);
    let inner: &dyn Evaluable = component.to_owned().value.unwrap();
    inner.evaluate(&arguments)
}

pub fn get_arguments<'a>(
    component: &yarrow::Component,
    graph_evaluation: &'a GraphEvaluation
) -> NodeArguments<'a> {

    let mut arguments = NodeArguments::new();
    component.arguments.iter().for_each(|(field_id, field)| {
        let evaluation: &'a FieldEvaluation = graph_evaluation.get(&field.source_node_id).unwrap().get(&field.source_field).unwrap().to_owned();
        arguments.insert(field_id.to_owned(), evaluation);
    });
    arguments
}

pub fn get_f64(arguments: &NodeArguments, column: &str) -> f64 {
    match arguments.get(column).unwrap() {
        FieldEvaluation::Bool(x) => Ok(if *x.first().unwrap() {1.} else {0.}),
        FieldEvaluation::I64(x) => Ok(f64::from(*x.first().unwrap() as i32)),
        FieldEvaluation::F64(x) => Ok(x.first().unwrap().to_owned()),
        _ => Err(column.to_string() +" must be numeric")
    }.unwrap()
}

pub fn get_array_f64(arguments: &NodeArguments, column: &str) -> ArrayD<f64> {
    match arguments.get(column).unwrap() {
        FieldEvaluation::Bool(x) => Ok(x.mapv(|v| if v {1.} else {0.})),
        FieldEvaluation::I64(x) => Ok(x.mapv(|v| f64::from(v as i32))),
        FieldEvaluation::F64(x) => Ok(x.to_owned()),
        _ => Err(column.to_string() +" must be numeric")
    }.unwrap()
}

pub fn release_to_evaluations(release: &yarrow::Release) -> GraphEvaluation {
    let mut evaluations = GraphEvaluation::new();

    for (node_id, node_release) in &release.values {
        let mut evaluations_node = NodeEvaluation::new();
        for (field_id, field_release) in &node_release.values {
            evaluations_node.insert(field_id.to_owned(), parse_proto_array(&field_release));
        }
        evaluations.insert(*node_id, evaluations_node);
    }
    evaluations
}

pub fn evaluations_to_release(evaluations: &GraphEvaluation) -> yarrow::Release {
    let mut releases = HashMap::new();
    for (node_id, node_eval) in evaluations {
        let mut node_release = HashMap::new();

        for (field_name, field_eval) in node_eval {
            node_release.insert(field_name.to_owned(), serialize_proto_array(&field_eval));
        }
        releases.insert(*node_id, yarrow::ReleaseNode {
            values: node_release.to_owned()
        });
    }
    yarrow::Release {
        values: releases
    }
}

pub fn parse_proto_array(value: &yarrow::ArrayNd) -> FieldEvaluation {
    let value = value.to_owned();
    let shape: Vec<usize> = value.shape.iter().map(|x| *x as usize).collect();
    match value.data.unwrap() {
        yarrow::array_nd::Data::Bytes(x) =>
            FieldEvaluation::Bytes(Array::from_shape_vec(shape, x).unwrap().into_dyn()),
        yarrow::array_nd::Data::Bool(x) =>
            FieldEvaluation::Bool(Array::from_shape_vec(shape, x.data).unwrap().into_dyn()),
        yarrow::array_nd::Data::I64(x) =>
            FieldEvaluation::I64(Array::from_shape_vec(shape, x.data).unwrap().into_dyn()),
        yarrow::array_nd::Data::F64(x) =>
            FieldEvaluation::F64(Array::from_shape_vec(shape, x.data).unwrap().into_dyn()),
        yarrow::array_nd::Data::String(x) =>
            FieldEvaluation::Str(Array::from_shape_vec(shape, x.data).unwrap().into_dyn()),
    }
}

pub fn serialize_proto_array(evaluation: &FieldEvaluation) -> yarrow::ArrayNd {

    match evaluation {
        FieldEvaluation::Bytes(x) => yarrow::ArrayNd {
            datatype: yarrow::DataType::Bytes as i32,
            data: Some(yarrow::array_nd::Data::Bytes(x.iter().map(|s| *s).collect())),
            order: (1..x.ndim()).map(|x| {x as u64}).collect(),
            shape: x.shape().iter().map(|y| {*y as u64}).collect()
        },
        FieldEvaluation::Bool(x) => yarrow::ArrayNd {
            datatype: yarrow::DataType::Bool as i32,
            data: Some(yarrow::array_nd::Data::Bool(yarrow::Array1Dbool {
                data: x.iter().map(|s| *s).collect()
            })),
            order: (1..x.ndim()).map(|x| {x as u64}).collect(),
            shape: x.shape().iter().map(|y| {*y as u64}).collect()
        },
        FieldEvaluation::I64(x) => yarrow::ArrayNd {
            datatype: yarrow::DataType::I64 as i32,
            data: Some(yarrow::array_nd::Data::I64(yarrow::Array1Di64 {
                data: x.iter().map(|s| *s).collect()
            })),
            order: (1..x.ndim()).map(|x| {x as u64}).collect(),
            shape: x.shape().iter().map(|y| {*y as u64}).collect()
        },
        FieldEvaluation::F64(x) => yarrow::ArrayNd {
            datatype: yarrow::DataType::F64 as i32,
            data: Some(yarrow::array_nd::Data::F64(yarrow::Array1Df64 {
                data: x.iter().map(|s| *s).collect()
            })),
            order: (1..x.ndim()).map(|x| {x as u64}).collect(),
            shape: x.shape().iter().map(|y| {*y as u64}).collect()
        },
        FieldEvaluation::Str(x) => yarrow::ArrayNd {
            datatype: yarrow::DataType::String as i32,
            data: Some(yarrow::array_nd::Data::String(yarrow::Array1Dstr {
                data: x.iter().cloned().collect()
            })),
            order: (1..x.ndim()).map(|x| {x as u64}).collect(),
            shape: x.shape().iter().map(|y| {*y as u64}).collect()
        },
    }
}
