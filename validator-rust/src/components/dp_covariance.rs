use crate::errors::*;


use std::collections::HashMap;

use crate::{proto, base};
use crate::hashmap;
use crate::components::{Component, Accuracy, Expandable, Report};


use crate::base::{NodeProperties, Value, ValueProperties, prepend};
use crate::utilities::json::{JSONRelease, value_to_json, AlgorithmInfo, privacy_usage_to_json};
use std::convert::TryFrom;


impl Component for proto::DpCovariance {
    fn propagate_property(
        &self,
        _privacy_definition: &proto::PrivacyDefinition,
        _public_arguments: &HashMap<String, Value>,
        _properties: &base::NodeProperties,
    ) -> Result<ValueProperties> {
        Err("DPCovariance is abstract, and has no property propagation".into())
    }

    fn get_names(
        &self,
        _properties: &NodeProperties,
    ) -> Result<Vec<String>> {
        Err("get_names not implemented".into())
    }
}


impl Expandable for proto::DpCovariance {
    fn expand_component(
        &self,
        _privacy_definition: &proto::PrivacyDefinition,
        component: &proto::Component,
        properties: &base::NodeProperties,
        component_id: u32,
        maximum_id: u32,
    ) -> Result<proto::ComponentExpansion> {
        let mut current_id = maximum_id.clone();
        let mut computation_graph: HashMap<u32, proto::Component> = HashMap::new();

        let arguments;
        let shape;
        let symmetric;
        match properties.get("data") {
            Some(data_property) => {
                let data_property = data_property.get_arraynd()
                    .map_err(prepend("data:"))?.clone();

                let num_columns = data_property.get_num_columns()?;
                shape = vec![u32::try_from(num_columns).unwrap(), u32::try_from(num_columns).unwrap()];
                arguments = hashmap![
                    "data".to_owned() => *component.arguments.get("data").ok_or::<Error>("data must be provided as an argument".into())?
                ];
                symmetric = true;
            },
            None => {
                let left_property = properties.get("left")
                    .ok_or("data: missing")?.get_arraynd()
                    .map_err(prepend("data:"))?.clone();
                let right_property = properties.get("right")
                    .ok_or("data: missing")?.get_arraynd()
                    .map_err(prepend("data:"))?.clone();

                shape = vec![u32::try_from(left_property.get_num_columns()?).unwrap(), u32::try_from(right_property.get_num_columns()?).unwrap()];
                arguments = hashmap![
                    "left".to_owned() => *component.arguments.get("left").ok_or::<Error>("left must be provided as an argument".into())?,
                    "right".to_owned() => *component.arguments.get("right").ok_or::<Error>("right must be provided as an argument".into())?
                ];
                symmetric = false;
            }
        };

        // covariance
        current_id += 1;
        let id_covariance = current_id.clone();
        computation_graph.insert(id_covariance, proto::Component {
            arguments,
            variant: Some(proto::component::Variant::from(proto::Covariance {
                finite_sample_correction: self.finite_sample_correction
            })),
            omit: true,
            batch: component.batch,
        });

        // noise
        current_id += 1;
        let id_noise = current_id.clone();
        computation_graph.insert(id_noise, proto::Component {
            arguments: hashmap!["data".to_owned() => id_covariance],
            variant: Some(proto::component::Variant::from(proto::LaplaceMechanism {
                privacy_usage: self.privacy_usage.clone()
            })),
            omit: true,
            batch: component.batch,
        });

        // reshape into matrix
        computation_graph.insert(component_id, proto::Component {
            arguments: hashmap!["data".to_owned() => id_noise],
            variant: Some(proto::component::Variant::from(proto::Reshape {
                symmetric,
                layout: "row".to_string(),
                shape
            })),
            omit: false,
            batch: component.batch
        });

        Ok(proto::ComponentExpansion {
            computation_graph,
            properties: HashMap::new(),
            releases: HashMap::new(),
            traversal: vec![id_covariance, id_noise]
        })
    }
}

impl Accuracy for proto::DpCovariance {
    fn accuracy_to_privacy_usage(
        &self,
        _privacy_definition: &proto::PrivacyDefinition,
        _properties: &base::NodeProperties,
        _accuracy: &proto::Accuracy,
    ) -> Option<proto::PrivacyUsage> {
        None
    }

    fn privacy_usage_to_accuracy(
        &self,
        _privacy_definition: &proto::PrivacyDefinition,
        _property: &base::NodeProperties,
    ) -> Option<f64> {
        None
    }
}

impl Report for proto::DpCovariance {
    fn summarize(
        &self,
        node_id: &u32,
        component: &proto::Component,
        _public_arguments: &HashMap<String, Value>,
        properties: &NodeProperties,
        release: &Value
    ) -> Result<Option<Vec<JSONRelease>>> {

        let argument;
        let statistic;

        if properties.contains_key("data") {
            let data_property = properties.get("data")
                .ok_or("data: missing")?.get_arraynd()
                .map_err(prepend("data:"))?.clone();

            statistic = "DPCovariance".to_string();
            argument = serde_json::json!({
                "n": data_property.get_num_records()?,
                "constraint": {
                    "lowerbound": data_property.get_min_f64()?,
                    "upperbound": data_property.get_max_f64()?
                }
            });
        }
        else {
            let left_property = properties.get("left")
                .ok_or("data: missing")?.get_arraynd()
                .map_err(prepend("data:"))?.clone();
            let right_property = properties.get("right")
                .ok_or("data: missing")?.get_arraynd()
                .map_err(prepend("data:"))?.clone();

            statistic = "DPCrossCovariance".to_string();
            argument = serde_json::json!({
                "n": left_property.get_num_records()?,
                "constraint": {
                    "lowerbound_left": left_property.get_min_f64()?,
                    "upperbound_left": left_property.get_max_f64()?,
                    "lowerbound_right": right_property.get_min_f64()?,
                    "upperbound_right": right_property.get_max_f64()?
                }
            });
        }

        let privacy_usage: Vec<serde_json::Value> = self.privacy_usage.iter()
            .map(privacy_usage_to_json).clone().collect();


        Ok(Some(vec![JSONRelease {
            description: "DP release information".to_string(),
            statistic,
            variables: serde_json::json!(Vec::<String>::new()),
            release_info: value_to_json(&release)?,
            privacy_loss: serde_json::json![privacy_usage],
            accuracy: None,
            batch: component.batch as u64,
            node_id: node_id.clone() as u64,
            postprocess: false,
            algorithm_info: AlgorithmInfo {
                name: "".to_string(),
                cite: "".to_string(),
                mechanism: self.implementation.clone(),
                argument
            }
        }]))
    }
}
