use whitenoise_validator::errors::*;

use crate::base::NodeArguments;
use whitenoise_validator::base::{Value, Array};
use whitenoise_validator::utilities::{get_argument, broadcast_privacy_usage, broadcast_ndarray};
use crate::components::Evaluable;
use crate::utilities;
use whitenoise_validator::proto;
use ndarray;

impl Evaluable for proto::LaplaceMechanism {
    fn evaluate(&self, arguments: &NodeArguments) -> Result<Value> {
        let mut data = match get_argument(&arguments, "data")?.array()? {
            Array::F64(data) => data.clone(),
            Array::I64(data) => data.mapv(|v| v as f64),
            _ => return Err("data must be numeric".into())
        };
        let sensitivity = get_argument(&arguments, "sensitivity")?.array()?.f64()?;

        let usages = broadcast_privacy_usage(&self.privacy_usage, sensitivity.len())?;

        let epsilon = ndarray::Array::from_shape_vec(
            data.shape(), usages.iter().map(get_epsilon).collect::<Result<Vec<f64>>>()?)?;

        data.gencolumns_mut().into_iter()
            .zip(sensitivity.gencolumns().into_iter().zip(epsilon.gencolumns().into_iter()))
            .map(|(mut data_column, (sensitivity, epsilon))| data_column.iter_mut()
                .zip(sensitivity.iter().zip(epsilon.iter()))
                .map(|(v, (sens, eps))| {
                    *v += utilities::mechanisms::laplace_mechanism(&eps, &sens)?;
                    Ok(())
                })
                .collect::<Result<()>>())
            .collect::<Result<()>>()?;

        Ok(data.into())
    }
}

impl Evaluable for proto::GaussianMechanism {
    fn evaluate(&self, arguments: &NodeArguments) -> Result<Value> {
        let mut data = match get_argument(&arguments, "data")?.array()? {
            Array::F64(data) => data.clone(),
            Array::I64(data) => data.mapv(|v| v as f64),
            _ => return Err("data must be numeric".into())
        };
//        println!("data: {:?}", data.shape());

        let sensitivity = get_argument(&arguments, "sensitivity")?.array()?.f64()?;
//        println!("sensitivity: {:?}", sensitivity.shape());

        let usages = broadcast_privacy_usage(&self.privacy_usage, sensitivity.len())?;

        let epsilon = ndarray::Array::from_shape_vec(
            data.shape(), usages.iter().map(get_epsilon).collect::<Result<Vec<f64>>>()?)?;
//        println!("epsilon: {:?}", epsilon.shape());

        let delta = ndarray::Array::from_shape_vec(
            data.shape(), usages.iter().map(get_delta).collect::<Result<Vec<f64>>>()?)?;
//        println!("delta: {:?}", delta.shape());

        data.gencolumns_mut().into_iter()
            .zip(sensitivity.gencolumns().into_iter())
            .zip(epsilon.gencolumns().into_iter().zip(delta.gencolumns().into_iter()))
            .map(|((mut data_column, sensitivity), (epsilon, delta))| data_column.iter_mut()
                .zip(sensitivity.iter())
                .zip(epsilon.iter().zip(delta.iter()))
                .map(|((v, sens), (eps, del))| {
                    *v += utilities::mechanisms::gaussian_mechanism(&eps, &del, &sens)?;
                    Ok(())
                }).collect::<Result<()>>())
            .collect::<Result<()>>()?;

        Ok(data.into())
    }
}

impl Evaluable for proto::SimpleGeometricMechanism {
    fn evaluate(&self, arguments: &NodeArguments) -> Result<Value> {
        let mut data = get_argument(&arguments, "data")?.array()?.i64()?.clone();
//        println!("data: {:?}", data.shape());

        let sensitivity = get_argument(&arguments, "sensitivity")?.array()?.f64()?;
//        println!("sensitivity: {:?}", sensitivity.shape());

        let usages = broadcast_privacy_usage(&self.privacy_usage, sensitivity.len())?;
        let epsilon = ndarray::Array::from_shape_vec(
            data.shape(), usages.iter().map(get_epsilon).collect::<Result<Vec<f64>>>()?)?;
//        println!("epsilon: {:?}", epsilon.shape());

        let min = broadcast_ndarray(
            get_argument(&arguments, "min")?.array()?.i64()?, data.shape())?;
//        println!("min: {:?}", min.shape());

        let max = broadcast_ndarray(
            get_argument(&arguments, "max")?.array()?.i64()?, data.shape())?;
//        println!("max: {:?}", max.shape());

        data.gencolumns_mut().into_iter()
            .zip(sensitivity.gencolumns().into_iter().zip(epsilon.gencolumns().into_iter()))
            .zip(min.gencolumns().into_iter().zip(max.gencolumns().into_iter()))
            .map(|((mut data_column, (sensitivity, epsilon)), (min, max))| data_column.iter_mut()
                .zip(sensitivity.iter().zip(epsilon.iter()))
                .zip(min.iter().zip(max.iter()))
                .map(|((v, (sens, eps)), (c_min, c_max))| {
                    *v += utilities::mechanisms::simple_geometric_mechanism(
                        &eps, &sens, &c_min, &c_max, &self.enforce_constant_time)?;
                    Ok(())
                })
                .collect::<Result<()>>())
            .collect::<Result<()>>()?;

        Ok(data.into())
    }
}


fn get_epsilon(usage: &proto::PrivacyUsage) -> Result<f64> {
    match usage.distance.clone()
        .ok_or_else(|| Error::from("distance must be defined on a PrivacyUsage"))? {
        proto::privacy_usage::Distance::DistancePure(distance) => Ok(distance.epsilon),
        proto::privacy_usage::Distance::DistanceApproximate(distance) => Ok(distance.epsilon),
//        _ => Err("epsilon is not defined".into())
    }
}

fn get_delta(usage: &proto::PrivacyUsage) -> Result<f64> {
    match usage.distance.clone()
        .ok_or_else(|| Error::from("distance must be defined on a PrivacyUsage"))? {
        proto::privacy_usage::Distance::DistanceApproximate(distance) => Ok(distance.delta),
        _ => Err("delta is not defined".into())
    }
}