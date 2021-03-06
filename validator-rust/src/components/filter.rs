use crate::errors::*;

use crate::components::Component;
use std::collections::HashMap;
use crate::base::{Value, ValueProperties, DataType};
use crate::utilities::prepend;
use crate::base;
use crate::proto;
use crate::components::transforms::propagate_binary_shape;

impl Component for proto::Filter {
    // modify min, max, n, categories, is_public, non-null, etc. based on the arguments and component
    fn propagate_property(
        &self,
        _privacy_definition: &proto::PrivacyDefinition,
        _public_arguments: &HashMap<String, Value>,
        properties: &base::NodeProperties,
    ) -> Result<ValueProperties> {
        let mut data_property = properties.get("data")
            .ok_or("data: missing")?.array()
            .map_err(prepend("data:"))?.clone();
        data_property.assert_is_not_aggregated()?;

        let mask_property = properties.get("mask")
            .ok_or("mask: missing")?.array()
            .map_err(prepend("mask:"))?.clone();
        mask_property.assert_is_not_aggregated()?;

        if mask_property.data_type != DataType::Bool {
            return Err("mask: must be boolean".into())
        }

        if mask_property.num_columns()? != 1 {
            return Err("mask: number of columns must be one".into())
        }

        propagate_binary_shape(&data_property, &mask_property)?;

        data_property.assert_is_not_aggregated()?;

        // the number of records is not known after filtering rows
        data_property.num_records = None;

        // This exists to prevent binary ops on non-conformable arrays from being approved
        data_property.dataset_id = None;

        Ok(data_property.into())
    }


}