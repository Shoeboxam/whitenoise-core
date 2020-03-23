use crate::errors::*;

use crate::components::Component;
use std::collections::HashMap;
use crate::base::{Value, prepend, ValueProperties};
use crate::base;
use crate::proto;

impl Component for proto::Reshape {
    fn propagate_property(
        &self,
        _privacy_definition: &proto::PrivacyDefinition,
        _public_arguments: &HashMap<String, Value>,
        properties: &base::NodeProperties,
    ) -> Result<ValueProperties> {
        let mut data_property = properties.get("data")
            .ok_or("data: missing")?.get_arraynd()
            .map_err(prepend("data:"))?.clone();

        if !data_property.releasable {
            return Err("data must be public/releasable to reshape".into())
        }

        data_property.num_records = match self.shape.len().clone() {
            0 => Some(1),
            1 | 2 => Some(self.shape[0] as i64),
            _ => return Err("dimensionality may not be greater than 2".into())
        };

        data_property.num_columns = match self.shape.len().clone() {
            0 | 1 => Some(1),
            2 => Some(self.shape[1] as i64),
            _ => return Err("dimensionality may not be greater than 2".into())
        };

        if data_property.num_records.unwrap() < 1 {
            return Err("number of records must be greater than zero".into())
        }
        if data_property.num_columns.unwrap() < 1 {
            return Err("number of columns must be greater than zero".into())
        }

        // Treat this as a new dataset, because number of rows is not necessarily the same anymore
        // This exists to prevent binary ops on non-conformable arrays from being approved
        data_property.dataset_id = None;

        Ok(data_property.into())
    }

    fn get_names(
        &self,
        _properties: &base::NodeProperties,
    ) -> Result<Vec<String>> {
        Err("get_names not implemented".into())
    }
}